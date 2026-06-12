import type { JSX } from 'react';
import type { AppWords } from '@velata/core';
import { useInsights } from '../hooks.js';
import type { SettingsApi } from '../hooks.js';
import { ipc } from '../ipc.js';
import { Toggle } from '../components/Toggle.js';

function Stat({ value, label }: { value: string; label: string }): JSX.Element {
  return (
    <div className="insight-stat">
      <div className="insight-value">{value}</div>
      <div className="insight-label">{label}</div>
    </div>
  );
}

/** One app's share of words as a labeled, proportional bar. */
function AppBar({ row, max }: { row: AppWords; max: number }): JSX.Element {
  const pct = max > 0 ? Math.round((row.words / max) * 100) : 0;
  return (
    <div className="insight-app-row">
      <div className="insight-app-head">
        <span className="insight-app-name">{row.name}</span>
        <span className="dict-from">{row.words.toLocaleString()}</span>
      </div>
      <div className="insight-bar-track">
        <div className="insight-bar-fill" style={{ width: `${String(pct)}%` }} />
      </div>
    </div>
  );
}

export function InsightsTab({ api }: { api: SettingsApi }): JSX.Element {
  const { insights, refresh } = useInsights();
  const modeName = (id: string): string => api.settings.modes.find((m) => m.id === id)?.name ?? id;

  // Prefer all-time figures when the user persists them; otherwise this session.
  const allTime = insights?.allTime ?? null;
  const scopeLabel = allTime ? 'All time' : 'This session';
  const words = allTime ? allTime.words : (insights?.totalWords ?? 0);
  const wpm = allTime ? allTime.wordsPerMinute : (insights?.wordsPerMinute ?? 0);
  const dictations = allTime ? allTime.dictations : (insights?.dictations ?? 0);
  const aiPercent = allTime ? allTime.aiPercent : (insights?.polishedPercent ?? 0);
  const fixes = allTime ? allTime.fixes : (insights?.dictionaryFixes ?? 0);

  const perApp = insights?.perApp ?? [];
  const maxAppWords = perApp.reduce((m, row) => Math.max(m, row.words), 0);
  const perAppScopeLabel = insights?.perAppScope === 'allTime' ? 'All time' : 'This session';

  const resetStats = (): void => {
    void ipc
      .clearInsights()
      .then(refresh)
      .catch((err: unknown) => {
        console.error('Failed to reset insights:', err);
      });
  };

  const hasActivity = dictations > 0 || words > 0;

  return (
    <div className="tab-body">
      <section className="card">
        <h2>Insights</h2>

        {!hasActivity ? (
          <p className="row-hint">
            No dictations yet. Hold <kbd>⌥Space</kbd> and speak to get started.
          </p>
        ) : (
          <>
            <div className="insights-grid insights-grid-3">
              <Stat value={words.toLocaleString()} label="words dictated" />
              <Stat value={String(wpm)} label="words / minute" />
              <Stat value={String(dictations)} label="dictations" />
              <Stat value={`${String(aiPercent)}%`} label="AI polished" />
              <Stat value={fixes.toLocaleString()} label="dictionary fixes" />
            </div>
            <p className="row-hint">{scopeLabel}</p>
          </>
        )}
      </section>

      {insights?.streak && (
        <section className="card">
          <h2>Streak</h2>
          <div className="home-stat-row">
            <span>Current streak</span>
            <span className="home-stat-value">
              {insights.streak.current} {insights.streak.current === 1 ? 'day' : 'days'}
            </span>
          </div>
          <div className="home-stat-row">
            <span>Longest streak</span>
            <span className="home-stat-value">
              {insights.streak.longest} {insights.streak.longest === 1 ? 'day' : 'days'}
            </span>
          </div>
        </section>
      )}

      <section className="card">
        <h2>Where it goes</h2>
        {perApp.length === 0 ? (
          <p className="row-hint">No per-app activity yet.</p>
        ) : (
          <>
            <div className="insight-apps">
              {perApp.map((row) => (
                <AppBar key={row.name} row={row} max={maxAppWords} />
              ))}
            </div>
            <p className="row-hint">{perAppScopeLabel}</p>
          </>
        )}
      </section>

      {insights && insights.topModes.length > 0 && (
        <section className="card">
          <h2>Most-used modes</h2>
          <div className="insights-modes">
            {insights.topModes.map((m) => (
              <div key={m.modeId} className="insights-mode-row">
                <span>{modeName(m.modeId)}</span>
                <span className="dict-from">{m.count}</span>
              </div>
            ))}
          </div>
        </section>
      )}

      {!api.settings.appStatsEnabled ? (
        <section className="card">
          <h2>All-time stats</h2>
          <p className="row-hint">
            Stores counts and dates — never your words or audio — only on this Mac.
          </p>
          <div className="row-actions">
            <Toggle
              checked={api.settings.appStatsEnabled}
              onChange={(checked) => void api.update({ appStatsEnabled: checked })}
              label="Keep all-time stats"
            />
          </div>
        </section>
      ) : (
        <section className="card">
          <h2>All-time stats</h2>
          <p className="row-hint">Lifetime totals and streaks are on for this Mac.</p>
          <div className="row-actions">
            <button className="btn btn-sm btn-danger" onClick={resetStats}>
              Reset all-time stats
            </button>
          </div>
        </section>
      )}

      <p className="row-hint">Computed and stored only on this Mac.</p>
    </div>
  );
}
