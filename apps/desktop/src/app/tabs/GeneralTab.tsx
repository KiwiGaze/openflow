import { useEffect, useState, type JSX, type ReactNode } from 'react';
import { type Appearance, formatAcceleratorMac } from '@velata/core';
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

/** History, stats, and notes: three local opt-ins, each with its own controls. */
function DataPrivacyCard({ api }: { api: SettingsApi }): JSX.Element {
  const { settings, update } = api;
  const [noteCount, setNoteCount] = useState<number | null>(null);
  const [clearError, setClearError] = useState<string | null>(null);

  // Only read notes when the Scratchpad is on — the command refuses while off,
  // and counting is the only reason to touch it here.
  useEffect(() => {
    if (!settings.scratchpadEnabled) {
      setNoteCount(null);
      return;
    }
    void ipc
      .listNotes(null)
      .then((notes) => {
        setNoteCount(notes.length);
      })
      .catch((err: unknown) => {
        console.error('Failed to count notes:', err);
      });
  }, [settings.scratchpadEnabled]);

  const clearHistory = (): void => {
    setClearError(null);
    void ipc.clearHistory().catch((err: unknown) => {
      // The Rust side reports success only once the rows are gone, so a
      // rejection means transcripts may still be on disk — say so.
      setClearError(`Couldn't clear history — it may still be on disk. ${String(err)}`);
    });
  };

  const resetStats = (): void => {
    void ipc.clearInsights().catch((err: unknown) => {
      console.error('Failed to reset stats:', err);
    });
  };

  return (
    <section className="card">
      <h2>Data &amp; privacy</h2>
      <p className="row-hint">
        By default Velata stores nothing. History, stats, and notes are separate opt-ins, kept only
        on this Mac.
      </p>

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
            Clear history
          </button>
        </div>
      )}
      {clearError && <p className="row-hint row-hint-warn">{clearError}</p>}

      <LocalRow
        title="Keep all-time stats"
        hint="Persist lifetime usage counts and dates — never your words or audio."
      >
        <Toggle
          checked={settings.appStatsEnabled}
          onChange={(checked) => void update({ appStatsEnabled: checked })}
          label="Keep all-time stats"
        />
      </LocalRow>
      {settings.appStatsEnabled && (
        <div className="row-actions">
          <button className="btn btn-quiet btn-danger" onClick={resetStats}>
            Reset stats
          </button>
        </div>
      )}

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
      {settings.scratchpadEnabled && (
        <div className="row-actions">
          {noteCount !== null && (
            <span className="row-hint">
              {noteCount} {noteCount === 1 ? 'note' : 'notes'} on this Mac
            </span>
          )}
          <button className="btn btn-quiet" onClick={() => void ipc.openScratchpadWindow(null)}>
            Open Scratchpad
          </button>
        </div>
      )}
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
        <Row title="Show in Dock" hint="Keep a Dock icon. Off keeps Velata in the menu bar only.">
          <Toggle
            checked={settings.showInDock}
            onChange={(checked) => void update({ showInDock: checked })}
            label="Show app in Dock"
          />
        </Row>
      </section>

      <DataPrivacyCard api={api} />

      <section className="card">
        <h2>Keyboard shortcuts</h2>
        <dl className="about-list">
          <dt>Dictate</dt>
          <dd>
            Hold {formatAcceleratorMac(settings.dictationHotkey)} (or tap to latch hands-free; tap
            again to stop)
          </dd>
          <dt>See changes</dt>
          <dd>
            {settings.changeOverlayHotkey
              ? `Tap ${formatAcceleratorMac(settings.changeOverlayHotkey)} to see what cleanup changed`
              : 'Off — set a hotkey on the Dictation page'}
          </dd>
          <dt>Polish selection</dt>
          <dd>
            {settings.polishHotkey ? `Tap ${formatAcceleratorMac(settings.polishHotkey)}` : 'Off'}{' '}
            (configured on the Transforms page)
          </dd>
          <dt>Run a mode</dt>
          <dd>Each mode can carry its own hotkey, set per mode on the Modes page</dd>
          <dt>Run a transform</dt>
          <dd>Each transform can carry its own hotkey, set per transform on the Transforms page</dd>
          <dt>Cancel or close</dt>
          <dd>Esc stops recording, or closes this window. Cmd+W also closes this window</dd>
          <dt>Copy last result</dt>
          <dd>From the menu bar, “Copy last dictation”</dd>
        </dl>
      </section>
    </div>
  );
}
