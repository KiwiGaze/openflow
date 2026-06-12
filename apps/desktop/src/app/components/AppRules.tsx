import { useEffect, useState, type JSX } from 'react';
import type { AppRule, CleanupLevel, FrontmostApp } from '@velata/core';
import type { SettingsApi } from '../hooks.js';
import { ipc } from '../ipc.js';

/** Add-form value for "no per-app override"; maps to `cleanupLevel: null`. */
const INHERIT = 'inherit';
type CleanupChoice = typeof INHERIT | CleanupLevel;

function choiceToLevel(choice: CleanupChoice): CleanupLevel | null {
  return choice === INHERIT ? null : choice;
}

/**
 * App rules card (07 §9, extended in 12): a frontmost app maps to a mode for
 * that job only, optionally overriding the global cleanup level for that app.
 */
export function AppRules({ api }: { api: SettingsApi }): JSX.Element {
  const { settings, update } = api;
  const [lastApp, setLastApp] = useState<FrontmostApp | null>(null);
  const [bundleId, setBundleId] = useState('');
  const [modeId, setModeId] = useState(settings.activeModeId);
  const [cleanup, setCleanup] = useState<CleanupChoice>(INHERIT);

  useEffect(() => {
    void ipc.getLastDictationApp().then(setLastApp);
  }, []);

  const addRule = (bundle: string): void => {
    const trimmed = bundle.trim();
    if (trimmed === '') return;
    // One rule per bundle id; replace any existing rule for it.
    const rules = settings.appRules.filter((r) => r.bundleId !== trimmed);
    void update({
      appRules: [...rules, { bundleId: trimmed, modeId, cleanupLevel: choiceToLevel(cleanup) }],
    });
    setBundleId('');
  };

  const patchRule = (bundle: string, patch: Partial<AppRule>): void => {
    void update({
      appRules: settings.appRules.map((r) => (r.bundleId === bundle ? { ...r, ...patch } : r)),
    });
  };

  const removeRule = (bundle: string): void => {
    void update({ appRules: settings.appRules.filter((r) => r.bundleId !== bundle) });
  };

  return (
    <section className="card">
      <h2>App rules</h2>
      <p className="row-hint">
        Dictate in a chosen mode automatically when an app is in front — one-shot, so your active
        mode never changes. The cleanup column can override the global level just for that app.
      </p>
      {settings.appRules.length > 0 && (
        <div className="dict-list">
          {settings.appRules.map((rule) => (
            <div key={rule.bundleId} className="dict-row">
              <span className="dict-to">{rule.bundleId}</span>
              <select
                aria-label={`Mode for ${rule.bundleId}`}
                value={rule.modeId}
                onChange={(e) => {
                  patchRule(rule.bundleId, { modeId: e.target.value });
                }}
              >
                {settings.modes.map((m) => (
                  <option key={m.id} value={m.id}>
                    {m.name}
                  </option>
                ))}
              </select>
              <select
                aria-label={`Cleanup for ${rule.bundleId}`}
                value={rule.cleanupLevel ?? INHERIT}
                onChange={(e) => {
                  patchRule(rule.bundleId, {
                    cleanupLevel: choiceToLevel(e.target.value as CleanupChoice),
                  });
                }}
              >
                <option value={INHERIT}>Inherit</option>
                <option value="off">Off</option>
                <option value="rules">Rules</option>
                <option value="ai">AI</option>
              </select>
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
          aria-label="Mode for the new rule"
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
        <select
          aria-label="Cleanup for the new rule"
          value={cleanup}
          onChange={(e) => {
            setCleanup(e.target.value as CleanupChoice);
          }}
        >
          <option value={INHERIT}>Inherit</option>
          <option value="off">Off</option>
          <option value="rules">Rules</option>
          <option value="ai">AI</option>
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
