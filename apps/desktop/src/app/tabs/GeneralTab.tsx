import { useState, type JSX, type ReactNode } from 'react';
import { type Appearance } from '@velata/core';
import type { SettingsApi } from '../hooks.js';
import { ipc } from '../ipc.js';
import { Row } from '../components/Row.js';
import { Toggle } from '../components/Toggle.js';

/** A flat row whose title carries an "On this Mac" locality badge. */
function LocalRow({
  title,
  hint,
  children,
}: {
  title: string;
  hint: string;
  children: ReactNode;
}): JSX.Element {
  return (
    <div className="row">
      <div className="row-text">
        <div className="row-title">
          {title}
          <span className="badge badge-local">On this Mac</span>
        </div>
        <div className="row-hint">{hint}</div>
      </div>
      <div className="row-control">{children}</div>
    </div>
  );
}

/** History and notes: local opt-ins, each with its own controls. */
function DataCard({ api }: { api: SettingsApi }): JSX.Element {
  const { settings, update } = api;
  const [clearError, setClearError] = useState<string | null>(null);

  const clearHistory = (): void => {
    setClearError(null);
    void ipc.clearHistory().catch((err: unknown) => {
      // The Rust side reports success only once the rows are gone, so a
      // rejection means transcripts may still be on disk — say so.
      setClearError(`Couldn't clear history — it may still be on disk. ${String(err)}`);
    });
  };

  return (
    <section className="card">
      <h2>Data</h2>
      <p className="row-hint">Stored only on this Mac.</p>

      <LocalRow
        title="Save history"
        hint="Keep a local, searchable log of past dictations (text only, never audio)."
      >
        <Toggle
          checked={settings.historyEnabled}
          onChange={(checked) => void update({ historyEnabled: checked })}
          label="Save history"
        />
      </LocalRow>
      {settings.historyEnabled && (
        <div className="row-actions">
          <select
            value={settings.historyRetentionDays}
            onChange={(e) => void update({ historyRetentionDays: Number(e.target.value) })}
          >
            <option value={0}>Keep forever</option>
            <option value={30}>30 days</option>
            <option value={7}>7 days</option>
            <option value={1}>1 day</option>
          </select>
          <button className="btn btn-quiet btn-danger" onClick={clearHistory}>
            Clear
          </button>
        </div>
      )}
      {clearError && <p className="row-hint row-hint-warn">{clearError}</p>}

      <LocalRow
        title="Scratchpad"
        hint="A local notes surface. Off, no note is written and every note command refuses."
      >
        <Toggle
          checked={settings.scratchpadEnabled}
          onChange={(checked) => void update({ scratchpadEnabled: checked })}
          label="Scratchpad"
        />
      </LocalRow>

      <p className="row-hint">
        Insights are always kept — counts &amp; dates only, never your words or audio.
      </p>
    </section>
  );
}

export function GeneralTab({ api }: { api: SettingsApi }): JSX.Element {
  const { settings, update } = api;

  return (
    <div className="tab-body">
      <section className="card">
        <h2>General</h2>
        <Row title="Launch at login" hint="Start Velata in the menu bar when you sign in.">
          <Toggle
            checked={settings.launchAtLogin}
            onChange={(checked) => void update({ launchAtLogin: checked })}
            label="Launch at login"
          />
        </Row>
        <Row title="Appearance" hint="Match macOS, or force light or dark for Velata's windows.">
          <select
            value={settings.appearance}
            onChange={(e) => void update({ appearance: e.target.value as Appearance })}
          >
            <option value="system">System</option>
            <option value="light">Light</option>
            <option value="dark">Dark</option>
          </select>
        </Row>
        <Row title="Show in Dock" hint="Keep a Dock icon. Off keeps Velata in the menu bar only.">
          <Toggle
            checked={settings.showInDock}
            onChange={(checked) => void update({ showInDock: checked })}
            label="Show app in Dock"
          />
        </Row>
      </section>

      <DataCard api={api} />
    </div>
  );
}
