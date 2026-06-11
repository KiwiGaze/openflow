import { useEffect, useState, type JSX } from 'react';
import type { HistoryEntry } from '@openflow/core';
import type { SettingsApi } from '../hooks.js';
import { ipc } from '../ipc.js';

/** The opt-in history view: search, reprocess through any mode, clear. */
export function History({ api }: { api: SettingsApi }): JSX.Element {
  const { settings } = api;
  const [entries, setEntries] = useState<HistoryEntry[]>([]);
  const [query, setQuery] = useState('');
  const [openId, setOpenId] = useState<string | null>(null);
  const [reprocessMode, setReprocessMode] = useState(settings.activeModeId);
  const [result, setResult] = useState<string | null>(null);

  useEffect(() => {
    void ipc.getHistory().then(setEntries);
  }, []);

  const q = query.trim().toLowerCase();
  const filtered = q
    ? entries.filter((e) => `${e.text} ${e.raw}`.toLowerCase().includes(q))
    : entries;

  const clear = (): void => {
    void ipc.clearHistory().then(() => {
      setEntries([]);
      setOpenId(null);
    });
  };

  const reprocess = async (text: string): Promise<void> => {
    setResult(null);
    try {
      setResult(await ipc.reprocessHistory(text, reprocessMode));
    } catch (err) {
      setResult(String(err));
    }
  };

  return (
    <section className="card">
      <h2>History</h2>
      <p className="row-hint">
        Stored only on this Mac, text only — the most recent dictations. Re-run any of them through
        a different mode.
      </p>
      <input
        type="text"
        placeholder="Search…"
        value={query}
        onChange={(e) => {
          setQuery(e.target.value);
        }}
      />
      {filtered.length === 0 ? (
        <p className="row-hint">
          {entries.length === 0 ? 'No dictations recorded yet.' : 'No matches.'}
        </p>
      ) : (
        <div className="history-list">
          {filtered.map((entry) => (
            <div key={entry.id} className="history-row">
              <p className="result-text">{entry.text}</p>
              <div className="row-actions">
                <span className="row-hint">{new Date(entry.at).toLocaleString()}</span>
                <button
                  className="btn btn-quiet btn-sm"
                  onClick={() => void navigator.clipboard.writeText(entry.text)}
                >
                  Copy
                </button>
                <button
                  className="btn btn-quiet btn-sm"
                  onClick={() => {
                    setOpenId(openId === entry.id ? null : entry.id);
                    setResult(null);
                  }}
                >
                  {openId === entry.id ? 'Close' : 'Reprocess'}
                </button>
              </div>
              {openId === entry.id && (
                <div className="row-actions">
                  <select
                    value={reprocessMode}
                    onChange={(e) => {
                      setReprocessMode(e.target.value);
                    }}
                  >
                    {settings.modes.map((m) => (
                      <option key={m.id} value={m.id}>
                        {m.name}
                      </option>
                    ))}
                  </select>
                  <button
                    className="btn btn-sm"
                    onClick={() => {
                      void reprocess(entry.text);
                    }}
                  >
                    Re-run
                  </button>
                  {result !== null && <p className="result-text">{result}</p>}
                </div>
              )}
            </div>
          ))}
        </div>
      )}
      <div className="row-actions">
        <button className="btn btn-quiet" disabled={entries.length === 0} onClick={clear}>
          Clear history
        </button>
      </div>
    </section>
  );
}
