import { useCallback, useEffect, useRef, useState, type ClipboardEvent, type JSX } from 'react';
import type { Note, NoteVersion } from '@velata/core';
import { events, ipc, subscribe } from './ipc.js';
import { useSettings } from './hooks.js';
import { Toggle } from './components/Toggle.js';
import {
  relativeTime,
  splitTransformBar,
  transformChips,
  versionLabel,
  type TransformChip,
} from './scratchpadView.js';

/** Milliseconds of idle typing before the editor flushes a save. */
const SAVE_DEBOUNCE_MS = 600;

/** The note id passed on the URL when the window is created (`?note=`). */
function initialNoteId(): string | null {
  return new URLSearchParams(window.location.search).get('note');
}

/**
 * The Scratchpad window: an opt-in, on-device notes surface in its own window.
 * Off, it shows only an enable card; on, a single-note editor (the note list
 * lives in the App window's Scratchpad tab) with versions before destructive
 * edits and transforms that reuse the one LLM client. All persistence is via
 * IPC; nothing is stored until the user turns the Scratchpad on.
 */
export function Scratchpad(): JSX.Element | null {
  const api = useSettings();

  useEffect(() => {
    document.documentElement.dataset.theme = api?.settings.appearance ?? 'system';
  }, [api?.settings.appearance]);

  if (!api) return null;

  if (!api.settings.scratchpadEnabled) {
    return (
      <div className="scratchpad-gate">
        <div className="scratchpad-gate-card">
          <h1 className="scratchpad-gate-title">Scratchpad</h1>
          <p className="scratchpad-gate-copy">
            Notes are stored only on this Mac, as text you can clear at any time. Nothing is saved
            until you turn Scratchpad on.
          </p>
          <Toggle
            checked={false}
            label="Turn on Scratchpad"
            onChange={() => {
              void api.update({ scratchpadEnabled: true });
            }}
          />
        </div>
      </div>
    );
  }

  return <Workspace transformChipList={transformChips(api.settings)} />;
}

/**
 * The enabled single-note window. It owns only the selected note id: it is
 * seeded from the URL and re-pointed when an already-open window is asked to
 * switch notes (`scratchpad-open-note`). The note list now lives in the App
 * window's Scratchpad tab, so there is no list pane here.
 *
 * The `key={selectedId}` on `<Editor>` is load-bearing: switching notes
 * unmounts the old editor, whose unmount effect flushes its pending debounced
 * save before the new note loads — so a card-click switch never drops an edit.
 */
function Workspace({ transformChipList }: { transformChipList: TransformChip[] }): JSX.Element {
  const [selectedId, setSelectedId] = useState<string | null>(initialNoteId());

  useEffect(() => {
    return subscribe(
      events.onScratchpadOpenNote((id) => {
        setSelectedId(id);
      }),
    );
  }, []);

  // No note: either opened without one (General's "Open Scratchpad") or the
  // current note was just deleted. Offer a way forward in-window.
  const createNote = (): void => {
    void ipc.createNote().then((note) => {
      setSelectedId(note.id);
    });
  };

  return (
    <div className="scratchpad scratchpad-single">
      {selectedId ? (
        <Editor
          key={selectedId}
          noteId={selectedId}
          transformChipList={transformChipList}
          onDeleted={() => {
            setSelectedId(null);
          }}
        />
      ) : (
        <div className="scratchpad-empty">
          <p>No note selected. Open one from the Scratchpad tab, or start a new one.</p>
          <button type="button" className="btn btn-primary" onClick={createNote}>
            New note
          </button>
        </div>
      )}
    </div>
  );
}

const TOOLBAR: { command: string; label: string; title: string }[] = [
  { command: 'bold', label: 'B', title: 'Bold' },
  { command: 'italic', label: 'I', title: 'Italic' },
  { command: 'underline', label: 'U', title: 'Underline' },
  { command: 'code', label: '</>', title: 'Inline code' },
  { command: 'insertUnorderedList', label: '• List', title: 'Bullet list' },
  { command: 'insertOrderedList', label: '1. List', title: 'Numbered list' },
];

function Editor({
  noteId,
  transformChipList,
  onDeleted,
}: {
  noteId: string;
  transformChipList: TransformChip[];
  onDeleted: (id: string) => void;
}): JSX.Element {
  const [note, setNote] = useState<Note | null>(null);
  const [title, setTitle] = useState('');
  const bodyRef = useRef<HTMLDivElement>(null);
  const dirtyRef = useRef(false);
  const timerRef = useRef<number | null>(null);
  const [busyTransform, setBusyTransform] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [showVersions, setShowVersions] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);

  // Load the note and seed the uncontrolled editor once. innerHTML is set here
  // (not on every keystroke) so typing never resets the caret.
  useEffect(() => {
    let cancelled = false;
    void ipc.getNote(noteId).then((loaded) => {
      if (cancelled || !loaded) return;
      setNote(loaded);
      setTitle(loaded.title);
      if (bodyRef.current) bodyRef.current.innerHTML = loaded.content;
    });
    return () => {
      cancelled = true;
    };
  }, [noteId]);

  // Resolves once the pending edit (if any) has committed, so callers that are
  // about to read the note server-side (transform, restore, delete) can await
  // it and never operate on stale content. Save failures are surfaced here and
  // the promise still resolves — ordering is what callers rely on. `node` lets
  // the unmount path read a body captured at effect setup: React 19 detaches
  // the object ref (bodyRef.current = null) during commit, before the passive
  // cleanup runs, so reading the ref then would flush an empty body and wipe
  // the note. A detached DOM node still retains its `.innerHTML`.
  const flush = useCallback(
    (node?: HTMLDivElement | null): Promise<void> => {
      if (timerRef.current !== null) {
        window.clearTimeout(timerRef.current);
        timerRef.current = null;
      }
      if (!dirtyRef.current) return Promise.resolve();
      dirtyRef.current = false;
      const content = (node ?? bodyRef.current)?.innerHTML ?? '';
      return ipc.updateNote(noteId, title, content).catch((err: unknown) => {
        setError(String(err));
      });
    },
    [noteId, title],
  );

  // Always reach the latest flush from the unmount effect without listing it as
  // a dependency — listing it would re-run (and prematurely flush) on every
  // title keystroke, defeating the debounce.
  const flushRef = useRef(flush);
  flushRef.current = flush;

  // Flush once on unmount — note switch (the parent re-keys by id) and window
  // close both unmount the editor, so no pending edit is lost. Capture the body
  // node at setup and pass it: by cleanup time React 19 has nulled the ref.
  useEffect(() => {
    const node = bodyRef.current;
    return () => {
      void flushRef.current(node);
    };
  }, []);

  const scheduleSave = useCallback((): void => {
    dirtyRef.current = true;
    if (timerRef.current !== null) window.clearTimeout(timerRef.current);
    timerRef.current = window.setTimeout(() => {
      void flushRef.current();
    }, SAVE_DEBOUNCE_MS);
  }, []);

  const onTitleChange = (value: string): void => {
    setTitle(value);
    scheduleSave();
  };

  // Paste as plain text only: bounds the stored HTML to our own toolbar's tags
  // (the body is re-rendered via innerHTML, so foreign markup would be an XSS
  // and styling hazard).
  const onPaste = (e: ClipboardEvent<HTMLDivElement>): void => {
    e.preventDefault();
    const text = e.clipboardData.getData('text/plain');
    // execCommand is the only API that inserts at the caret with native undo;
    // its deprecation has no replacement for contentEditable editing yet.
    // eslint-disable-next-line @typescript-eslint/no-deprecated
    document.execCommand('insertText', false, text);
    scheduleSave();
  };

  const runCommand = (command: string): void => {
    bodyRef.current?.focus();
    if (command === 'code') {
      wrapSelectionInCode();
    } else {
      // eslint-disable-next-line @typescript-eslint/no-deprecated
      document.execCommand(command);
    }
    scheduleSave();
  };

  const runTransform = (transformId: string | null): void => {
    setError(null);
    setBusyTransform(transformId ?? 'polish');
    // Await the flush so the transform reads the just-typed content, not the
    // last committed save.
    void flush()
      .then(() => ipc.transformNoteText(noteId, transformId))
      .then((updated) => {
        setNote(updated);
        setTitle(updated.title);
        if (bodyRef.current) bodyRef.current.innerHTML = updated.content;
      })
      .catch((err: unknown) => {
        setError(String(err));
      })
      .finally(() => {
        setBusyTransform(null);
      });
  };

  const copyNote = (): void => {
    const text = bodyRef.current?.textContent ?? '';
    void ipc.copyText(text);
  };

  const deleteNote = (): void => {
    void flush()
      .then(() => ipc.deleteNote(noteId))
      .then(() => {
        onDeleted(noteId);
      });
  };

  const restored = (updated: Note): void => {
    setNote(updated);
    setTitle(updated.title);
    if (bodyRef.current) bodyRef.current.innerHTML = updated.content;
  };

  if (!note) return <div className="scratchpad-editor" />;

  return (
    <div className="scratchpad-editor">
      <input
        type="text"
        className="scratchpad-title"
        placeholder="Untitled"
        value={title}
        onChange={(e) => {
          onTitleChange(e.target.value);
        }}
      />
      <div className="scratchpad-toolbar" role="toolbar" aria-label="Formatting">
        {TOOLBAR.map((item) => (
          <button
            key={item.command}
            type="button"
            className="scratchpad-tool"
            title={item.title}
            aria-label={item.title}
            onMouseDown={(e) => {
              // Keep the selection while clicking a tool button.
              e.preventDefault();
            }}
            onClick={() => {
              runCommand(item.command);
            }}
          >
            {item.label}
          </button>
        ))}
        <span className="scratchpad-toolbar-spacer" />
        <button
          type="button"
          className={`btn btn-quiet btn-sm ${showVersions ? 'scratchpad-tool-on' : ''}`}
          onClick={() => {
            setShowVersions((v) => !v);
          }}
        >
          History
        </button>
      </div>
      <div className="scratchpad-body-wrap">
        <div
          ref={bodyRef}
          className="scratchpad-body"
          contentEditable
          suppressContentEditableWarning
          role="textbox"
          aria-multiline="true"
          aria-label="Note body"
          onInput={scheduleSave}
          onPaste={onPaste}
          onBlur={() => {
            void flush();
          }}
        />
        {showVersions && (
          <VersionPanel
            noteId={noteId}
            transformChipList={transformChipList}
            flush={flush}
            onRestored={restored}
          />
        )}
      </div>
      {error && (
        <p className="scratchpad-error" role="alert">
          {error}
        </p>
      )}
      <div className="scratchpad-transforms">
        <TransformBar
          chips={transformChipList}
          busyTransform={busyTransform}
          onRun={runTransform}
        />
        <span className="scratchpad-actions-spacer" />
        <button type="button" className="btn btn-quiet btn-sm" onClick={copyNote}>
          Copy
        </button>
        {confirmDelete ? (
          <span className="scratchpad-confirm">
            <span className="scratchpad-confirm-label">Delete this note?</span>
            <button
              type="button"
              className="btn btn-quiet btn-sm"
              onClick={() => {
                setConfirmDelete(false);
              }}
            >
              Cancel
            </button>
            <button type="button" className="btn btn-danger btn-sm" onClick={deleteNote}>
              Delete
            </button>
          </span>
        ) : (
          <button
            type="button"
            className="btn btn-danger btn-sm"
            onClick={() => {
              setConfirmDelete(true);
            }}
          >
            Delete
          </button>
        )}
      </div>
    </div>
  );
}

/** A chip's busy key matches `runTransform`'s `transformId ?? 'polish'`. */
function chipKey(chip: TransformChip): string {
  return chip.id ?? 'polish';
}

function chipLabel(chip: TransformChip, busyTransform: string | null): string {
  return busyTransform === chipKey(chip) ? `${chip.label}…` : chip.label;
}

/**
 * The bottom transform bar: Polish and the first 3 transforms as inline chips,
 * the rest behind a "⋯ More" menu. The menu closes on outside click or Escape;
 * every item is disabled while a transform runs, like the inline chips.
 */
function TransformBar({
  chips,
  busyTransform,
  onRun,
}: {
  chips: TransformChip[];
  busyTransform: string | null;
  onRun: (transformId: string | null) => void;
}): JSX.Element {
  const { visible, overflow } = splitTransformBar(chips);
  const [menuOpen, setMenuOpen] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!menuOpen) return;
    const onPointerDown = (e: PointerEvent): void => {
      if (!menuRef.current?.contains(e.target as Node)) setMenuOpen(false);
    };
    const onKeyDown = (e: KeyboardEvent): void => {
      if (e.key === 'Escape') setMenuOpen(false);
    };
    document.addEventListener('pointerdown', onPointerDown);
    document.addEventListener('keydown', onKeyDown);
    return () => {
      document.removeEventListener('pointerdown', onPointerDown);
      document.removeEventListener('keydown', onKeyDown);
    };
  }, [menuOpen]);

  return (
    <>
      {visible.map((chip) => (
        <button
          key={chipKey(chip)}
          type="button"
          className="scratchpad-chip"
          disabled={busyTransform !== null}
          onClick={() => {
            onRun(chip.id);
          }}
        >
          {chipLabel(chip, busyTransform)}
        </button>
      ))}
      {overflow.length > 0 && (
        <div className="scratchpad-more" ref={menuRef}>
          <button
            type="button"
            className="scratchpad-chip"
            aria-haspopup="menu"
            aria-expanded={menuOpen}
            disabled={busyTransform !== null}
            onClick={() => {
              setMenuOpen((open) => !open);
            }}
          >
            ⋯ More ▾
          </button>
          {menuOpen && (
            <div className="scratchpad-more-menu" role="menu">
              {overflow.map((chip) => (
                <button
                  key={chipKey(chip)}
                  type="button"
                  role="menuitem"
                  className="scratchpad-more-item"
                  disabled={busyTransform !== null}
                  onClick={() => {
                    setMenuOpen(false);
                    onRun(chip.id);
                  }}
                >
                  {chipLabel(chip, busyTransform)}
                </button>
              ))}
            </div>
          )}
        </div>
      )}
    </>
  );
}

function VersionPanel({
  noteId,
  transformChipList,
  flush,
  onRestored,
}: {
  noteId: string;
  transformChipList: TransformChip[];
  /** The editor's pending-edit flush; awaited before a restore so the typed
   * (not yet debounce-saved) content is committed — and therefore captured in
   * the "restore" snapshot — before the server swaps it out. */
  flush: () => Promise<void>;
  onRestored: (note: Note) => void;
}): JSX.Element {
  const [versions, setVersions] = useState<NoteVersion[]>([]);
  // versionLabel needs the full transform list; rebuild it from the chips
  // (Polish carries a null id, so it is dropped here). flatMap narrows the id to
  // a string without a non-null assertion.
  const transforms = transformChipList.flatMap((c) =>
    c.id === null ? [] : [{ id: c.id, name: c.label, instruction: '', hotkey: '', builtIn: false }],
  );

  const refresh = useCallback((): void => {
    void ipc.listNoteVersions(noteId).then(setVersions);
  }, [noteId]);

  useEffect(() => {
    refresh();
    return subscribe(events.onNotesChanged(refresh));
  }, [refresh]);

  const restore = (versionId: string): void => {
    void flush()
      .then(() => ipc.restoreNoteVersion(versionId))
      .then(onRestored);
  };

  return (
    <div className="scratchpad-versions">
      <div className="scratchpad-versions-head">History</div>
      {versions.length === 0 ? (
        <p className="scratchpad-versions-empty">No earlier versions yet.</p>
      ) : (
        versions.map((v) => (
          <div key={v.id} className="scratchpad-version">
            <div className="scratchpad-version-meta">
              <span className="scratchpad-version-label">
                {versionLabel(v.source, v.transformId, transforms)}
              </span>
              <span className="scratchpad-version-time">
                {relativeTime(v.createdAt, Date.now())}
              </span>
            </div>
            <button
              type="button"
              className="btn btn-quiet btn-sm"
              onClick={() => {
                restore(v.id);
              }}
            >
              Restore
            </button>
          </div>
        ))
      )}
    </div>
  );
}

/**
 * Wraps the current selection in `<code>`, escaping it so the inserted markup is
 * exactly our own tag. execCommand has no inline-code command, so this is the
 * one place we build HTML by hand — only ever a single `<code>` element.
 */
function wrapSelectionInCode(): void {
  const selection = window.getSelection();
  if (!selection || selection.rangeCount === 0) return;
  const text = selection.toString();
  if (text === '') return;
  const escaped = text.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
  // eslint-disable-next-line @typescript-eslint/no-deprecated
  document.execCommand('insertHTML', false, `<code>${escaped}</code>`);
}
