import { useCallback, useEffect, useRef, useState, type ClipboardEvent, type JSX } from 'react';
import type { Note, NoteSummary, NoteVersion } from '@velata/core';
import { events, ipc, subscribe } from './ipc.js';
import { useSettings } from './hooks.js';
import { Toggle } from './components/Toggle.js';
import {
  noteTitle,
  relativeTime,
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
 * Off, it shows only an enable card; on, a two-pane notes manager (list + rich
 * text editor) with versions before destructive edits and transforms that reuse
 * the one LLM client. All persistence is via IPC; nothing is stored until the
 * user turns the Scratchpad on.
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

/** The enabled two-pane workspace. Split out so the gate stays a clean early return. */
function Workspace({ transformChipList }: { transformChipList: TransformChip[] }): JSX.Element {
  const [notes, setNotes] = useState<NoteSummary[]>([]);
  const [query, setQuery] = useState('');
  const [selectedId, setSelectedId] = useState<string | null>(initialNoteId());

  const refresh = useCallback((search: string): void => {
    void ipc.listNotes(search.trim() === '' ? null : search.trim()).then(setNotes);
  }, []);

  useEffect(() => {
    refresh(query);
  }, [query, refresh]);

  // The list is the durable source of truth; refresh on any note mutation, and
  // follow an external "open this note" request (from the tray/main window).
  useEffect(() => {
    const cleanups = [
      subscribe(
        events.onNotesChanged(() => {
          refresh(query);
        }),
      ),
      subscribe(
        events.onScratchpadOpenNote((id) => {
          setSelectedId(id);
        }),
      ),
    ];
    return () => {
      cleanups.forEach((fn) => {
        fn();
      });
    };
  }, [query, refresh]);

  const createNote = (): void => {
    void ipc.createNote().then((note) => {
      setSelectedId(note.id);
    });
  };

  // After a delete, land on the next note in the list (or the empty state).
  const handleDeleted = (deletedId: string): void => {
    const remaining = notes.filter((n) => n.id !== deletedId);
    setSelectedId(remaining[0]?.id ?? null);
  };

  return (
    <div className="scratchpad">
      <aside className="scratchpad-list">
        <div className="scratchpad-list-head">
          <input
            type="text"
            className="scratchpad-search"
            placeholder="Search notes"
            value={query}
            onChange={(e) => {
              setQuery(e.target.value);
            }}
          />
          <button type="button" className="btn btn-primary btn-sm" onClick={createNote}>
            New note
          </button>
        </div>
        <div className="scratchpad-rows">
          {notes.length === 0 ? (
            <p className="scratchpad-empty-list">
              {query.trim() === '' ? 'No notes yet.' : 'No matching notes.'}
            </p>
          ) : (
            notes.map((note) => (
              <NoteRow
                key={note.id}
                note={note}
                active={note.id === selectedId}
                onSelect={() => {
                  setSelectedId(note.id);
                }}
              />
            ))
          )}
        </div>
      </aside>
      <section className="scratchpad-editor-pane">
        {selectedId ? (
          <Editor
            key={selectedId}
            noteId={selectedId}
            transformChipList={transformChipList}
            onDeleted={handleDeleted}
          />
        ) : (
          <div className="scratchpad-empty">
            <p>Select a note, or start a new one.</p>
            <button type="button" className="btn btn-primary" onClick={createNote}>
              New note
            </button>
          </div>
        )}
      </section>
    </div>
  );
}

/**
 * One list row: two sibling buttons (open + pin) in a plain container — never a
 * control nested inside a control, so the pin is keyboard-reachable and both
 * get the global button focus ring. The pin mirrors the Toggle switch pattern.
 */
function NoteRow({
  note,
  active,
  onSelect,
}: {
  note: NoteSummary;
  active: boolean;
  onSelect: () => void;
}): JSX.Element {
  return (
    <div className={`scratchpad-row ${active ? 'scratchpad-row-active' : ''}`}>
      <button type="button" className="scratchpad-row-open" onClick={onSelect}>
        <span className="scratchpad-row-title">{noteTitle(note.title)}</span>
        {note.preview && <span className="scratchpad-row-preview">{note.preview}</span>}
        <span className="scratchpad-row-date">{relativeTime(note.updatedAt, Date.now())}</span>
      </button>
      <button
        type="button"
        role="switch"
        aria-checked={note.pinned}
        aria-label={note.pinned ? 'Unpin note' : 'Pin note'}
        className={`scratchpad-pin ${note.pinned ? 'scratchpad-pin-on' : ''}`}
        onClick={() => {
          void ipc.setNotePinned(note.id, !note.pinned);
        }}
      >
        {note.pinned ? '★' : '☆'}
      </button>
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
  // the promise still resolves — ordering is what callers rely on.
  const flush = useCallback((): Promise<void> => {
    if (timerRef.current !== null) {
      window.clearTimeout(timerRef.current);
      timerRef.current = null;
    }
    if (!dirtyRef.current) return Promise.resolve();
    dirtyRef.current = false;
    const content = bodyRef.current?.innerHTML ?? '';
    return ipc.updateNote(noteId, title, content).catch((err: unknown) => {
      setError(String(err));
    });
  }, [noteId, title]);

  // Always reach the latest flush from the unmount effect without listing it as
  // a dependency — listing it would re-run (and prematurely flush) on every
  // title keystroke, defeating the debounce.
  const flushRef = useRef(flush);
  flushRef.current = flush;

  // Flush once on unmount — note switch (the parent re-keys by id) and window
  // close both unmount the editor, so no pending edit is lost.
  useEffect(() => {
    return () => {
      void flushRef.current();
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
        {transformChipList.map((chip) => (
          <button
            key={chip.id ?? 'polish'}
            type="button"
            className="scratchpad-chip"
            disabled={busyTransform !== null}
            onClick={() => {
              runTransform(chip.id);
            }}
          >
            {busyTransform === (chip.id ?? 'polish') ? `${chip.label}…` : chip.label}
          </button>
        ))}
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
