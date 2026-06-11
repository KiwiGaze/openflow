import type { JSX } from 'react';
import {
  type Appearance,
  formatBytes,
  formatProgress,
  type HotkeyBehavior,
  type InsertMethod,
} from '@openflow/core';
import type { ModelsApi, SettingsApi } from '../hooks.js';
import { usePipeline } from '../hooks.js';
import { HotkeyRecorder } from '../components/HotkeyRecorder.js';
import { Row } from '../components/Row.js';
import { Toggle } from '../components/Toggle.js';

const LANGUAGES: [string, string][] = [
  ['auto', 'Auto-detect'],
  ['en', 'English'],
  ['zh', 'Chinese'],
  ['es', 'Spanish'],
  ['fr', 'French'],
  ['de', 'German'],
  ['ja', 'Japanese'],
  ['ko', 'Korean'],
  ['pt', 'Portuguese'],
  ['ru', 'Russian'],
  ['it', 'Italian'],
  ['nl', 'Dutch'],
  ['hi', 'Hindi'],
  ['ar', 'Arabic'],
];

export function GeneralTab({
  api,
  modelsApi,
}: {
  api: SettingsApi;
  modelsApi: ModelsApi;
}): JSX.Element {
  const { settings, update } = api;
  const { models, progress, download, cancel, remove } = modelsApi;
  const { lastResult } = usePipeline();

  return (
    <div className="tab-body">
      <section className="card">
        <h2>Hotkeys</h2>
        <Row
          title="Dictation"
          hint="Hold and speak; release to insert. A quick tap latches hands-free mode."
        >
          <HotkeyRecorder
            value={settings.dictationHotkey}
            onChange={(accelerator) => void update({ dictationHotkey: accelerator })}
          />
        </Row>
        <Row title="Dictation style" hint="How the dictation hotkey behaves.">
          <select
            value={settings.dictationHotkeyBehavior}
            onChange={(e) =>
              void update({ dictationHotkeyBehavior: e.target.value as HotkeyBehavior })
            }
          >
            <option value="hold">Hold to talk</option>
            <option value="toggle">Press to start / stop</option>
          </select>
        </Row>
        <Row
          title="Polish selection"
          hint="Fix grammar and clarity in the selected text. No voice."
        >
          <HotkeyRecorder
            value={settings.polishHotkey}
            onChange={(accelerator) => void update({ polishHotkey: accelerator })}
          />
        </Row>
        <Row title="Rewrite selection" hint="Select text anywhere, hold, and say what to change.">
          <HotkeyRecorder
            value={settings.refineHotkey}
            onChange={(accelerator) => void update({ refineHotkey: accelerator })}
          />
        </Row>
      </section>

      <section className="card">
        <h2>Speech recognition</h2>
        <div className="model-list">
          {models.map((model) => {
            const p = progress[model.id];
            const downloading = (model.downloading || (p && !p.done)) ?? false;
            const active = settings.sttModelId === model.id;
            return (
              <div key={model.id} className={`model-row ${active ? 'model-active' : ''}`}>
                <label className="model-pick">
                  <input
                    type="radio"
                    name="stt-model"
                    checked={active}
                    disabled={!model.installed}
                    onChange={() => void update({ sttModelId: model.id })}
                  />
                  <div>
                    <div className="row-title">
                      {model.displayName}
                      {model.multilingual && <span className="badge">multilingual</span>}
                    </div>
                    <div className="row-hint">
                      {formatBytes(model.sizeBytes)} — {model.description}
                    </div>
                  </div>
                </label>
                <div className="model-actions">
                  {model.installed && !active && (
                    <button className="btn btn-quiet" onClick={() => void remove(model.id)}>
                      Delete
                    </button>
                  )}
                  {model.installed && <span className="badge badge-ok">installed</span>}
                  {!model.installed && downloading && (
                    <>
                      <span className="row-hint">
                        {p ? formatProgress(p.downloadedBytes, p.totalBytes) : '…'}
                      </span>
                      <button
                        className="btn btn-quiet"
                        onClick={() => {
                          cancel(model.id);
                        }}
                      >
                        Cancel
                      </button>
                    </>
                  )}
                  {!model.installed && !downloading && (
                    <button
                      className="btn"
                      onClick={() => {
                        download(model.id);
                      }}
                    >
                      Download
                    </button>
                  )}
                </div>
              </div>
            );
          })}
        </div>
        <Row title="Spoken language" hint="English-only models ignore this.">
          <select
            value={settings.language}
            onChange={(e) => void update({ language: e.target.value })}
          >
            {LANGUAGES.map(([code, name]) => (
              <option key={code} value={code}>
                {name}
              </option>
            ))}
          </select>
        </Row>
      </section>

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
        <Row title="Restore clipboard" hint="Put your previous clipboard back after pasting.">
          <Toggle
            checked={settings.restoreClipboard}
            onChange={(checked) => void update({ restoreClipboard: checked })}
            label="Restore clipboard"
          />
        </Row>
        <Row title="Launch at login" hint="Start OpenFlow in the menu bar when you sign in.">
          <Toggle
            checked={settings.launchAtLogin}
            onChange={(checked) => void update({ launchAtLogin: checked })}
            label="Launch at login"
          />
        </Row>
        <Row title="Appearance" hint="Match macOS, or force light or dark for OpenFlow's windows.">
          <select
            value={settings.appearance}
            onChange={(e) => void update({ appearance: e.target.value as Appearance })}
          >
            <option value="system">System</option>
            <option value="light">Light</option>
            <option value="dark">Dark</option>
          </select>
        </Row>
      </section>

      {lastResult && (
        <section className="card">
          <h2>Last result</h2>
          <p className="result-text">{lastResult.text}</p>
          {lastResult.refined && lastResult.raw !== lastResult.text && (
            <p className="row-hint">Raw transcript: {lastResult.raw}</p>
          )}
          <button
            className="btn"
            onClick={() => void navigator.clipboard.writeText(lastResult.text)}
          >
            Copy
          </button>
        </section>
      )}
    </div>
  );
}
