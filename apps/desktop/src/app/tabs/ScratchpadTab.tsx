import { useEffect, useState, type JSX } from 'react';
import type { NoteSummary } from '@velata/core';
import { events, ipc, subscribe } from '../ipc.js';
import { Toggle } from '../components/Toggle.js';
import { noteCountLine } from '../scratchpadView.js';
import type { SettingsApi } from '../hooks.js';

/**
 * The Scratchpad entry in the settings window. Off, it is an enable gate; on, it
 * is a quiet launcher — a note count and an "Open Scratchpad" button. The notes
 * themselves live in their own window, so there is no manager UI duplicated here.
 */
export function ScratchpadTab({ api }: { api: SettingsApi }): JSX.Element {
  const { settings } = api;
  const [notes, setNotes] = useState<NoteSummary[]>([]);

  // Only the enabled tab needs a count; list on mount and refresh on changes.
  useEffect(() => {
    if (!settings.scratchpadEnabled) return;
    const load = (): void => {
      void ipc.listNotes(null).then(setNotes);
    };
    load();
    return subscribe(events.onNotesChanged(load));
  }, [settings.scratchpadEnabled]);

  if (!settings.scratchpadEnabled) {
    return (
      <div className="tab-body">
        <h1>Scratchpad</h1>
        <section className="card">
          <div className="row">
            <div>
              <div className="row-title">Turn on Scratchpad</div>
              <p className="row-hint">
                Notes are stored only on this Mac, as text you can clear at any time. Nothing is
                saved until you turn Scratchpad on.
              </p>
            </div>
            <div className="row-control">
              <Toggle
                checked={false}
                label="Turn on Scratchpad"
                onChange={() => {
                  void api.update({ scratchpadEnabled: true });
                }}
              />
            </div>
          </div>
        </section>
      </div>
    );
  }

  return (
    <div className="tab-body">
      <h1>Scratchpad</h1>
      <section className="card">
        <div className="row">
          <div>
            <div className="row-title">{noteCountLine(notes)}</div>
            <p className="row-hint">Your notes open in their own window.</p>
          </div>
          <div className="row-control">
            <button
              type="button"
              className="btn btn-primary"
              onClick={() => {
                void ipc.openScratchpadWindow(null);
              }}
            >
              Open Scratchpad
            </button>
          </div>
        </div>
      </section>
    </div>
  );
}
