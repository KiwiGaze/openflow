import type { JSX } from 'react';
import { type Appearance } from '@openflow/core';
import type { SettingsApi } from '../hooks.js';
import { Row } from '../components/Row.js';
import { Toggle } from '../components/Toggle.js';

export function GeneralTab({ api }: { api: SettingsApi }): JSX.Element {
  const { settings, update } = api;

  return (
    <div className="tab-body">
      <section className="card">
        <h2>General</h2>
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
        <Row title="Welcome tour" hint="Replay the first-run setup guide.">
          <button className="btn" onClick={() => void update({ onboardingCompleted: false })}>
            Show again
          </button>
        </Row>
        <Row
          title="Show tips"
          hint="One-time hints about features you haven't tried. Never repeats, never interrupts dictation."
        >
          <Toggle
            checked={settings.tipsEnabled}
            onChange={(checked) => void update({ tipsEnabled: checked })}
            label="Show tips"
          />
        </Row>
        <div className="row-actions">
          <button
            className="btn btn-quiet"
            onClick={() => void update({ tipsSeen: [], lastTipShownAt: '' })}
          >
            Reset tips
          </button>
        </div>
      </section>
    </div>
  );
}
