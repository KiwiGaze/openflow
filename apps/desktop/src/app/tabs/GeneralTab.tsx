import type { JSX } from 'react';
import { type Appearance, formatAcceleratorMac } from '@openflow/core';
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
        <Row
          title="Save history"
          hint="Keep a local, searchable log of past dictations on this Mac (text only, never audio). Off by default. Clear it anytime under Output."
        >
          <Toggle
            checked={settings.historyEnabled}
            onChange={(checked) => void update({ historyEnabled: checked })}
            label="Save history"
          />
        </Row>
        <Row title="Show in Dock" hint="Keep a Dock icon. Off keeps OpenFlow in the menu bar only.">
          <Toggle
            checked={settings.showInDock}
            onChange={(checked) => void update({ showInDock: checked })}
            label="Show app in Dock"
          />
        </Row>
      </section>

      <section className="card">
        <h2>Keyboard shortcuts</h2>
        <dl className="about-list">
          <dt>Dictate</dt>
          <dd>
            Hold {formatAcceleratorMac(settings.dictationHotkey)} (or tap to latch hands-free; tap
            again to stop)
          </dd>
          <dt>Cancel</dt>
          <dd>Esc while recording</dd>
          <dt>Polish selection</dt>
          <dd>Tap {formatAcceleratorMac(settings.polishHotkey)}</dd>
          <dt>Copy last result</dt>
          <dd>From the menu bar, “Copy last dictation”</dd>
        </dl>
      </section>
    </div>
  );
}
