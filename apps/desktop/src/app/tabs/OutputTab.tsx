import type { JSX } from 'react';
import { type InsertMethod } from '@openflow/core';
import type { SettingsApi } from '../hooks.js';
import { usePipeline } from '../hooks.js';
import { History } from '../components/History.js';
import { Row } from '../components/Row.js';
import { Toggle } from '../components/Toggle.js';

export function OutputTab({ api }: { api: SettingsApi }): JSX.Element {
  const { settings, update } = api;
  const { lastResult } = usePipeline();

  return (
    <div className="tab-body">
      <section className="card">
        <h2>Output</h2>
        <Row title="Insert method" hint="Paste needs the Accessibility permission.">
          <select
            value={settings.insertMethod}
            onChange={(e) => void update({ insertMethod: e.target.value as InsertMethod })}
          >
            <option value="paste">Paste into the active app</option>
            <option value="clipboard">Copy to clipboard only</option>
          </select>
        </Row>
        <Row
          title="Restore clipboard"
          hint="After pasting, put back whatever you'd copied before. Turn off to keep the dictated text on the clipboard."
        >
          <Toggle
            checked={settings.restoreClipboard}
            onChange={(checked) => void update({ restoreClipboard: checked })}
            label="Restore clipboard"
          />
        </Row>
      </section>

      <section className="card">
        <h2>Last result</h2>
        {lastResult ? (
          <>
            <p className="result-text">{lastResult.text}</p>
            {lastResult.refined && lastResult.raw !== lastResult.text && (
              <p className="row-hint">Raw transcript: {lastResult.raw}</p>
            )}
            <button
              className="btn"
              onClick={() => {
                navigator.clipboard.writeText(lastResult.text).catch((err: unknown) => {
                  console.error('Copy failed:', err);
                });
              }}
            >
              Copy
            </button>
          </>
        ) : (
          <p className="row-hint">Your last dictation will appear here.</p>
        )}
      </section>

      {settings.historyEnabled && <History api={api} />}
    </div>
  );
}
