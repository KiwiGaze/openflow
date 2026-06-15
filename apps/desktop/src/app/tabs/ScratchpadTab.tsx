import { useEffect, useState, type JSX } from 'react';
import type { NoteSummary } from '@velata/core';
import { events, ipc, subscribe } from '../ipc.js';
import { noteTitle, relativeTime } from '../scratchpadView.js';
import type { SettingsApi } from '../hooks.js';

/**
 * The Scratchpad entry in the App window. Off, it points the user to the
 * Settings → General toggle (the only place Scratchpad is turned on). On, it is
 * the note list — a searchable card grid; clicking a card opens that note in the
 * single-note Scratchpad window, and "+ New" creates one and opens it.
 */
export function ScratchpadTab({ api }: { api: SettingsApi }): JSX.Element {
  const { settings } = api;
  const [notes, setNotes] = useState<NoteSummary[]>([]);
  const [query, setQuery] = useState('');

  // The durable rows are the source of truth: list on mount and whenever the
  // search changes, and refresh on any note mutation (create/delete/transform).
  useEffect(() => {
    if (!settings.scratchpadEnabled) return;
    const load = (): void => {
      const search = query.trim();
      void ipc.listNotes(search === '' ? null : search).then(setNotes);
    };
    load();
    return subscribe(events.onNotesChanged(load));
  }, [settings.scratchpadEnabled, query]);

  if (!settings.scratchpadEnabled) {
    return (
      <div className="tab-body">
        <h1>Scratchpad</h1>
        <section className="card">
          <div className="row">
            <div>
              <div className="row-title">Scratchpad is off</div>
              <p className="row-hint">
                Notes are stored only on this Mac, as text you can clear at any time. Turn
                Scratchpad on in Settings → General to start taking notes.
              </p>
            </div>
            <div className="row-control">
              <button
                type="button"
                className="btn btn-primary"
                onClick={() => {
                  void ipc.openSettingsWindow('general');
                }}
              >
                Open Settings
              </button>
            </div>
          </div>
        </section>
      </div>
    );
  }

  const createNote = (): void => {
    void ipc.createNote().then((note) => {
      void ipc.openScratchpadWindow(note.id);
    });
  };

  return (
    <div className="tab-body">
      <header className="transform-header">
        <div className="transform-header-text">
          <h1 className="transform-title">Scratchpad</h1>
          <p className="row-hint">Click a note to open it in its own window.</p>
        </div>
      </header>

      <div className="transform-toolbar">
        <input
          type="search"
          className="transform-search"
          placeholder="Search notes"
          aria-label="Search notes"
          value={query}
          onChange={(e) => {
            setQuery(e.target.value);
          }}
        />
        <button type="button" className="btn btn-primary" onClick={createNote}>
          + New
        </button>
      </div>

      {notes.length === 0 ? (
        <p className="row-hint">{query.trim() === '' ? 'No notes yet.' : 'No matches.'}</p>
      ) : (
        <div className="note-grid">
          {notes.map((note) => (
            <button
              key={note.id}
              type="button"
              className="note-card"
              onClick={() => {
                void ipc.openScratchpadWindow(note.id);
              }}
            >
              <span className="note-card-head">
                {note.pinned && (
                  <span className="note-card-pin" role="img" aria-label="Pinned">
                    📌
                  </span>
                )}
                <span className="note-card-title">{noteTitle(note.title)}</span>
              </span>
              {note.preview && <span className="note-card-preview">{note.preview}</span>}
              <span className="note-card-date">{relativeTime(note.updatedAt, Date.now())}</span>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
