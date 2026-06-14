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

type ModeName = (modeId: string) => string;

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
  const modeName: ModeName = (modeId) =>
    api.settings.modes.find((m) => m.id === modeId)?.name ?? modeId;

  return (
    <div className="tab-body">
      <header className="home-greeting">
        <h1>{greeting}</h1>
        <p className="row-hint">{dateFormat.format(now)}</p>
      </header>

      <div className="home-grid">
        <div className="home-main">
          {api.settings.historyEnabled ? (
            <HistorySection modeName={modeName} api={api} />
          ) : (
            <HistoryOffCard api={api} />
          )}
        </div>
        <aside className="home-aside">
          <UsageCard />
          <SetupCard modelsApi={modelsApi} onNavigate={onNavigate} />
        </aside>
      </div>
    </div>
  );
}

function HistorySection({ modeName, api }: { modeName: ModeName; api: SettingsApi }): JSX.Element {
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
              <HistoryRow
                key={entry.id}
                entry={entry}
                modeName={modeName}
                modes={api.settings.modes}
                activeModeId={api.settings.activeModeId}
                onChanged={refresh}
              />
            ))}
          </div>
        ))
      )}
    </section>
  );
}

function HistoryRow({
  entry,
  modeName,
  modes,
  activeModeId,
  onChanged,
}: {
  entry: HistoryEntry;
  modeName: ModeName;
  modes: SettingsApi['settings']['modes'];
  activeModeId: string;
  onChanged: () => void;
}): JSX.Element {
  const [open, setOpen] = useState(false);
  const [reprocessMode, setReprocessMode] = useState(activeModeId);
  const [running, setRunning] = useState(false);
  const [result, setResult] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const copy = (text: string): void => {
    void ipc.copyText(text).catch((err: unknown) => {
      console.error('Copy failed:', err);
    });
  };

  const rerun = (): void => {
    setRunning(true);
    setError(null);
    setResult(null);
    void ipc
      .reprocessHistory(entry.text, reprocessMode)
      .then(setResult)
      .catch((err: unknown) => {
        setError(String(err));
      })
      .finally(() => {
        setRunning(false);
      });
  };

  const remove = (): void => {
    void ipc
      .deleteHistoryEntry(entry.id)
      .then(onChanged)
      .catch((err: unknown) => {
        setError(`Couldn't delete this entry. ${String(err)}`);
      });
  };

  return (
    <div className="home-entry">
      <button
        type="button"
        className="home-entry-head"
        aria-expanded={open}
        onClick={() => {
          setOpen(!open);
        }}
      >
        <span className="home-entry-preview">{entry.text}</span>
        <span className="home-entry-meta">
          <span className="home-entry-time">{timeFormat.format(entry.at)}</span>
          {entry.appName && <span className="home-entry-app">{entry.appName}</span>}
          <span className="badge">{modeName(entry.modeId)}</span>
        </span>
      </button>

      {open && (
        <div className="home-entry-detail">
          <div className="row-title">Raw transcript</div>
          <p className="result-text">{entry.rawText}</p>

          <div className="row-actions">
            <button
              className="btn btn-sm"
              onClick={() => {
                copy(entry.text);
              }}
            >
              Copy
            </button>
            <select
              aria-label="Re-run through mode"
              value={reprocessMode}
              onChange={(e) => {
                setReprocessMode(e.target.value);
              }}
            >
              {modes.map((m) => (
                <option key={m.id} value={m.id}>
                  {m.name}
                </option>
              ))}
            </select>
            <button className="btn btn-sm" disabled={running} onClick={rerun}>
              {running ? 'Running…' : 'Run'}
            </button>
            <button className="btn btn-sm btn-danger" onClick={remove}>
              Delete
            </button>
          </div>

          {error && <p className="row-hint row-hint-warn">{error}</p>}
          {result !== null && (
            <div className="home-entry-result">
              <p className="result-text">{result}</p>
              <div className="row-actions">
                <button
                  className="btn btn-sm"
                  onClick={() => {
                    copy(result);
                  }}
                >
                  Copy
                </button>
              </div>
            </div>
          )}
        </div>
      )}
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

function UsageCard(): JSX.Element {
  const { insights } = useInsights();

  return (
    <section className="card">
      <h2>Usage</h2>
      {!insights || insights.dictations === 0 ? (
        <p className="row-hint">No dictations yet this session.</p>
      ) : (
        <>
          <div className="home-stat-row">
            <span>Words</span>
            <span className="home-stat-value">{insights.totalWords.toLocaleString()}</span>
          </div>
          <div className="home-stat-row">
            <span>Words / min</span>
            <span className="home-stat-value">{insights.wordsPerMinute}</span>
          </div>
          <div className="home-stat-row">
            <span>Dictations</span>
            <span className="home-stat-value">{insights.dictations}</span>
          </div>
          {insights.polishedPercent > 0 && (
            <div className="home-stat-row">
              <span>AI-polished</span>
              <span className="home-stat-value">{insights.polishedPercent}%</span>
            </div>
          )}
          <p className="row-hint">This session</p>
        </>
      )}
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
