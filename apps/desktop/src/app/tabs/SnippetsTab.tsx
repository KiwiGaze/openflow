import { useState, type JSX } from 'react';
import { validateSnippet } from '@openflow/core';
import { Toggle } from '../components/Toggle.js';
import type { SettingsApi } from '../hooks.js';

/** Collapses whitespace and truncates a multi-line expansion for the list. */
function preview(expansion: string): string {
  const oneLine = expansion.replace(/\s+/g, ' ').trim();
  return oneLine.length > 80 ? `${oneLine.slice(0, 80)}…` : oneLine;
}

export function SnippetsTab({ api }: { api: SettingsApi }): JSX.Element {
  const { settings, save } = api;
  const [trigger, setTrigger] = useState('');
  const [expansion, setExpansion] = useState('');
  const [wholeUtterance, setWholeUtterance] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const add = (): void => {
    const snippet = { trigger: trigger.trim(), expansion, wholeUtterance };
    const problem = validateSnippet(snippet, settings.snippets);
    if (problem) {
      setError(problem);
      return;
    }
    setError(null);
    setTrigger('');
    setExpansion('');
    setWholeUtterance(false);
    void save({ ...settings, snippets: [...settings.snippets, snippet] });
  };

  const removeAt = (index: number): void => {
    void save({
      ...settings,
      snippets: settings.snippets.filter((_, i) => i !== index),
    });
  };

  return (
    <div className="tab-body">
      <section className="card">
        <h2>Snippets</h2>
        <p className="row-hint">
          Say a short phrase, insert a longer block — an email address, a link, a canned reply.
          Triggers match whole words and ignore case; the expansion is inserted exactly as written,
          never reworded by AI.
        </p>
        <form
          className="snippet-form"
          onSubmit={(e) => {
            e.preventDefault();
            add();
          }}
        >
          <input
            type="text"
            className="snippet-trigger"
            placeholder="When I say… (e.g. “my email”)"
            value={trigger}
            onChange={(e) => {
              setTrigger(e.target.value);
            }}
          />
          <textarea
            className="snippet-expansion"
            placeholder="Insert this… (e.g. “me@example.com”)"
            rows={3}
            value={expansion}
            onChange={(e) => {
              setExpansion(e.target.value);
            }}
          />
          <div className="snippet-form-actions">
            <div className="snippet-scope">
              <Toggle
                checked={wholeUtterance}
                onChange={setWholeUtterance}
                label="Expand only when spoken alone"
              />
              <div className="snippet-scope-text">
                <span>Only when spoken alone</span>
                <span className="row-hint">Stops it expanding mid-sentence.</span>
              </div>
            </div>
            <button className="btn btn-primary" type="submit">
              Add snippet
            </button>
          </div>
        </form>
        {error && <p className="form-error">{error}</p>}

        {settings.snippets.length === 0 ? (
          <p className="row-hint">No snippets yet.</p>
        ) : (
          <div className="dict-list">
            {settings.snippets.map((snippet, index) => (
              <div key={`${snippet.trigger}-${index}`} className="snippet-row">
                <div className="snippet-row-text">
                  <div className="snippet-row-head">
                    <span className="dict-from">{snippet.trigger}</span>
                    {snippet.wholeUtterance && <span className="snippet-tag">alone</span>}
                  </div>
                  <span className="snippet-preview">{preview(snippet.expansion)}</span>
                </div>
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
