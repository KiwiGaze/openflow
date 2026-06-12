import type { JSX } from 'react';
import { useInsights } from '../hooks.js';
import type { SettingsApi } from '../hooks.js';

function Stat({ value, label }: { value: string; label: string }): JSX.Element {
  return (
    <div className="insight-stat">
      <div className="insight-value">{value}</div>
      <div className="insight-label">{label}</div>
    </div>
  );
}

export function InsightsTab({ api }: { api: SettingsApi }): JSX.Element {
  const insights = useInsights();
  const modeName = (id: string): string => api.settings.modes.find((m) => m.id === id)?.name ?? id;

  return (
    <div className="tab-body">
      <section className="card">
        <h2>Insights</h2>
        <p className="row-hint">
          Your usage this session — tallied in memory on this Mac, never written to disk or
          uploaded, and reset on quit. (Dictation only touches the network if you opt in: a cloud AI
          profile, or a model download.)
        </p>

        {!insights || insights.dictations === 0 ? (
          <p className="row-hint">
            No dictations yet this session. Hold <kbd>⌥Space</kbd> and speak to get started.
          </p>
        ) : (
          <>
            <div className="insights-grid">
              <Stat value={insights.totalWords.toLocaleString()} label="words dictated" />
              <Stat value={String(insights.wordsPerMinute)} label="words / minute" />
              <Stat value={String(insights.dictations)} label="dictations" />
              <Stat value={`${String(insights.polishedPercent)}%`} label="AI-polished" />
            </div>

            {insights.topModes.length > 0 && (
              <div className="insights-modes">
                <div className="row-title">Most-used modes</div>
                {insights.topModes.map((m) => (
                  <div key={m.modeId} className="insights-mode-row">
                    <span>{modeName(m.modeId)}</span>
                    <span className="dict-from">{m.count}</span>
                  </div>
                ))}
              </div>
            )}
          </>
        )}
      </section>
    </div>
  );
}
