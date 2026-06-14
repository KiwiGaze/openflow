import { useCallback, useEffect, useState, type JSX } from 'react';
import type { HistoryEntry, PermissionsState } from '@velata/core';
import { useInsights, type ModelsApi, type SettingsApi } from '../hooks.js';
import { events, ipc, subscribe } from '../ipc.js';
import { greetingForHour, groupHistoryByDay } from '../homeData.js';
import type { TabId } from '../sidebarTabs.js';
import { Toggle } from '../components/Toggle.js';

const timeFormat = new Intl.DateTimeFormat(undefined, { hour: '2-digit', minute: '2-digit' });
const dateFormat = new Intl.DateTimeFormat(undefined, {
  weekday: 'long',
  month: 'long',
  day: 'numeric',
  year: 'numeric',
});

export function HomeTab({
  api,
  modelsApi,
  onNavigate,
}: {
  api: SettingsApi;
  modelsApi: ModelsApi;
  onNavigate: (tab: TabId) => void;
}): JSX.Element {
  const now = new Date();
  const greeting = greetingForHour(now.getHours());

  return (
    <div className="tab-body">
      <header className="home-greeting">
        <h1>{greeting}</h1>
        <p className="row-hint">{dateFormat.format(now)}</p>
      </header>

      <HomeStats />

      <div className="home-grid">
        <div className="home-main">
          {api.settings.historyEnabled ? <HistorySection /> : <HistoryOffCard api={api} />}
        </div>
        <aside className="home-aside">
          <SetupCard modelsApi={modelsApi} onNavigate={onNavigate} />
        </aside>
      </div>
    </div>
  );
}

/** Four-up activity summary; hidden until at least one dictation is recorded. */
function HomeStats(): JSX.Element | null {
  const { insights } = useInsights();

  if (!insights || insights.dictations === 0) {
    return null;
  }

  return (
    <div className="home-stats">
      <Stat value={insights.words.toLocaleString()} label="words" />
      <Stat value={insights.dictations.toLocaleString()} label="dictations" />
      <Stat value={insights.wordsPerMinute.toLocaleString()} label="wpm" />
      <Stat value={insights.streak.toLocaleString()} label="day streak" />
    </div>
  );
}

function Stat({ value, label }: { value: string; label: string }): JSX.Element {
  return (
    <div className="home-stat">
      <div className="home-stat-number">{value}</div>
      <div className="home-stat-label">{label}</div>
    </div>
  );
}

function HistorySection(): JSX.Element {
  const [entries, setEntries] = useState<HistoryEntry[]>([]);

  const refresh = useCallback(() => {
    void ipc
      .getHistory()
      .then(setEntries)
      .catch((err: unknown) => {
        console.error('Failed to load history:', err);
      });
  }, []);

  useEffect(() => {
    refresh();
    // history-changed fires after the DB append commits, so this reads durable
    // rows instead of racing the write (which onResult would).
    return subscribe(events.onHistoryChanged(refresh));
  }, [refresh]);

  const groups = groupHistoryByDay(entries, new Date());

  return (
    <section className="card">
      <h2>History</h2>
      {entries.length === 0 ? (
        <p className="row-hint">Dictations you make will appear here.</p>
      ) : (
        groups.map((group) => (
          <div key={group.key} className="home-day">
            <div className="home-day-label">{group.label}</div>
            {group.entries.map((entry) => (
              <HistoryRow key={entry.id} entry={entry} onChanged={refresh} />
            ))}
          </div>
        ))
      )}
    </section>
  );
}

function HistoryRow({
  entry,
  onChanged,
}: {
  entry: HistoryEntry;
  onChanged: () => void;
}): JSX.Element {
  const copy = (): void => {
    void ipc.copyText(entry.text).catch((err: unknown) => {
      console.error('Copy failed:', err);
    });
  };

  const remove = (): void => {
    void ipc
      .deleteHistoryEntry(entry.id)
      .then(onChanged)
      .catch((err: unknown) => {
        console.error('Delete failed:', err);
      });
  };

  return (
    <div className="home-entry">
      <span className="home-entry-preview">{entry.text}</span>
      <span className="home-entry-meta">
        <span className="home-entry-time">{timeFormat.format(entry.at)}</span>
        <button className="btn btn-sm" onClick={copy}>
          Copy
        </button>
        <button className="btn btn-sm btn-danger" onClick={remove}>
          Delete
        </button>
      </span>
    </div>
  );
}

function HistoryOffCard({ api }: { api: SettingsApi }): JSX.Element {
  return (
    <section className="card">
      <h2>History</h2>
      <p className="row-hint">
        History is stored only on this Mac, as text you can clear at any time. Nothing is stored
        until you turn it on.
      </p>
      <div className="row-actions">
        <Toggle
          checked={api.settings.historyEnabled}
          onChange={(checked) => void api.update({ historyEnabled: checked })}
          label="Save history"
        />
      </div>
    </section>
  );
}

function SetupCard({
  modelsApi,
  onNavigate,
}: {
  modelsApi: ModelsApi;
  onNavigate: (tab: TabId) => void;
}): JSX.Element | null {
  const [permissions, setPermissions] = useState<PermissionsState | null>(null);

  useEffect(() => {
    void ipc.checkPermissions().then(setPermissions);
  }, []);

  const micMissing = permissions !== null && permissions.microphone !== 'granted';
  const modelMissing = !modelsApi.models.some((m) => m.installed);

  if (!micMissing && !modelMissing) {
    return null;
  }

  return (
    <section className="card">
      <h2>Set up</h2>
      {micMissing && (
        <SetupRow
          label="Allow microphone access"
          actionLabel="Open"
          onClick={() => {
            onNavigate('dictation');
          }}
        />
      )}
      {modelMissing && (
        <SetupRow
          label="Download a speech model"
          actionLabel="Speech"
          onClick={() => {
            onNavigate('speech');
          }}
        />
      )}
    </section>
  );
}

function SetupRow({
  label,
  actionLabel,
  onClick,
}: {
  label: string;
  actionLabel: string;
  onClick: () => void;
}): JSX.Element {
  return (
    <div className="home-setup-row">
      <span>{label}</span>
      <button className="btn btn-sm" onClick={onClick}>
        {actionLabel}
      </button>
    </div>
  );
}
