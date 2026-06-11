import { useEffect, useState, type JSX } from 'react';
import type { FrontmostApp } from '@openflow/core';
import type { SettingsApi } from '../hooks.js';
import { ipc } from '../ipc.js';

/** App rules card (07 §9): a frontmost app maps to a mode for that job only. */
export function AppRules({ api }: { api: SettingsApi }): JSX.Element {
  const { settings, update } = api;
  const [lastApp, setLastApp] = useState<FrontmostApp | null>(null);
  const [bundleId, setBundleId] = useState('');
  const [modeId, setModeId] = useState(settings.activeModeId);

  useEffect(() => {
    void ipc.getLastDictationApp().then(setLastApp);
  }, []);

  const addRule = (bundle: string): void => {
    const trimmed = bundle.trim();
    if (trimmed === '') return;
    // One rule per bundle id; replace any existing rule for it.
    const rules = settings.appRules.filter((r) => r.bundleId !== trimmed);
    void update({ appRules: [...rules, { bundleId: trimmed, modeId }] });
    setBundleId('');
  };

  const removeRule = (bundle: string): void => {
    void update({ appRules: settings.appRules.filter((r) => r.bundleId !== bundle) });
  };

  const modeName = (id: string): string => settings.modes.find((m) => m.id === id)?.name ?? id;

  return (
    <section className="card">
      <h2>App rules</h2>
      <p className="row-hint">
        Dictate in a chosen mode automatically when an app is in front — one-shot, so your active
        mode never changes.
      </p>
      {settings.appRules.length > 0 && (
        <div className="dict-list">
          {settings.appRules.map((rule) => (
            <div key={rule.bundleId} className="dict-row">
              <span className="dict-from">{rule.bundleId}</span>
              <span className="dict-arrow">→</span>
              <span className="dict-to">{modeName(rule.modeId)}</span>
              <button
                className="btn btn-quiet btn-sm"
                onClick={() => {
                  removeRule(rule.bundleId);
                }}
              >
                Remove
              </button>
            </div>
          ))}
        </div>
      )}
      <div className="row-actions">
        <input
          type="text"
          placeholder="Bundle id (e.g. com.apple.Notes)"
          value={bundleId}
          onChange={(e) => {
            setBundleId(e.target.value);
          }}
        />
        <select
          value={modeId}
          onChange={(e) => {
            setModeId(e.target.value);
          }}
        >
          {settings.modes.map((m) => (
            <option key={m.id} value={m.id}>
              {m.name}
            </option>
          ))}
        </select>
        <button
          className="btn"
          onClick={() => {
            addRule(bundleId);
          }}
        >
          Add rule
        </button>
      </div>
      {lastApp && !settings.appRules.some((r) => r.bundleId === lastApp.bundleId) && (
        <p className="row-hint">
          Last dictated into {lastApp.name} ({lastApp.bundleId}).{' '}
          <button
            className="btn btn-quiet btn-sm"
            onClick={() => {
              addRule(lastApp.bundleId);
            }}
          >
            Add a rule for it
          </button>
        </p>
      )}
    </section>
  );
}
