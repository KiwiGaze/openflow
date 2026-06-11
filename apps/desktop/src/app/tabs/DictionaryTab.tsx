import { useState, type JSX } from 'react';
import { validateDictionaryEntry } from '@openflow/core';
import { useDictionarySuggestions } from '../hooks.js';
import type { SettingsApi } from '../hooks.js';

export function DictionaryTab({ api }: { api: SettingsApi }): JSX.Element {
  const { settings, save } = api;
  const { suggestions, dismiss, refresh } = useDictionarySuggestions();
  const [from, setFrom] = useState('');
  const [to, setTo] = useState('');
  const [error, setError] = useState<string | null>(null);

  const add = (): void => {
    const entry = { from: from.trim(), to: to.trim() };
    const problem = validateDictionaryEntry(entry, settings.dictionary);
    if (problem) {
      setError(problem);
      return;
    }
    setError(null);
    setFrom('');
    setTo('');
    void save({ ...settings, dictionary: [...settings.dictionary, entry] });
  };

  const removeAt = (index: number): void => {
    void save({
      ...settings,
      dictionary: settings.dictionary.filter((_, i) => i !== index),
    });
  };

  // A suggested term is a vocabulary hint: keep this spelling (from === to).
  const accept = (term: string): void => {
    void save({
      ...settings,
      dictionary: [...settings.dictionary, { from: term, to: term }],
    }).then(refresh);
  };

  return (
    <div className="tab-body">
      <section className="card">
        <h2>Personal dictionary</h2>
        <p className="row-hint">
          Fix words the transcriber keeps getting wrong — names, products, jargon. Replacements
          match whole words, ignore case, and are also fed to the speech model as vocabulary hints.
        </p>

        {suggestions.length > 0 && (
          <div className="dict-suggestions">
            <div className="row-hint">
              Noticed while you spoke — add the ones you want kept spelled this way (this session
              only; nothing was saved).
            </div>
            <div className="dict-suggestion-chips">
              {suggestions.map((s) => (
                <span key={s.term} className="dict-suggestion">
                  <button
                    type="button"
                    className="chip"
                    title={`Seen ${String(s.count)}× — add to dictionary`}
                    onClick={() => {
                      accept(s.term);
                    }}
                  >
                    + {s.term}
                  </button>
                  <button
                    type="button"
                    className="chip-dismiss"
                    aria-label={`Dismiss ${s.term}`}
                    onClick={() => {
                      dismiss(s.term);
                    }}
                  >
                    ×
                  </button>
                </span>
              ))}
            </div>
          </div>
        )}

        <form
          className="dict-form"
          onSubmit={(e) => {
            e.preventDefault();
            add();
          }}
        >
          <input
            type="text"
            placeholder="Heard as… (e.g. “open flow”)"
            value={from}
            onChange={(e) => {
              setFrom(e.target.value);
            }}
          />
          <span className="dict-arrow">→</span>
          <input
            type="text"
            placeholder="Replace with… (e.g. “OpenFlow”)"
            value={to}
            onChange={(e) => {
              setTo(e.target.value);
            }}
          />
          <button className="btn" type="submit">
            Add
          </button>
        </form>
        {error && <p className="form-error">{error}</p>}

        {settings.dictionary.length === 0 ? (
          <p className="row-hint">No entries yet.</p>
        ) : (
          <div className="dict-list">
            {settings.dictionary.map((entry, index) => (
              <div key={`${entry.from}-${index}`} className="dict-row">
                {entry.from === entry.to ? (
                  <>
                    <span className="dict-to">{entry.to}</span>
                    <span className="badge badge-muted">kept as-is</span>
                  </>
                ) : (
                  <>
                    <span className="dict-from">{entry.from}</span>
                    <span className="dict-arrow">→</span>
                    <span className="dict-to">{entry.to}</span>
                  </>
                )}
                <button
                  className="btn btn-quiet"
                  onClick={() => {
                    removeAt(index);
                  }}
                >
                  Remove
                </button>
              </div>
            ))}
          </div>
        )}
      </section>
    </div>
  );
}
