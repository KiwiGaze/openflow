import { useRef, useState, type JSX } from 'react';
import { validateSnippet, type Snippet } from '@velata/core';
import { Toggle } from '../components/Toggle.js';
import { filterSnippets } from '../snippetView.js';
import type { SettingsApi } from '../hooks.js';

/** Collapses whitespace and truncates a multi-line expansion for the list. */
function preview(expansion: string): string {
  const oneLine = expansion.replace(/\s+/g, ' ').trim();
  return oneLine.length > 80 ? `${oneLine.slice(0, 80)}…` : oneLine;
}

export function SnippetsTab({ api }: { api: SettingsApi }): JSX.Element {
  const { settings, save } = api;
  const [query, setQuery] = useState('');
  const [trigger, setTrigger] = useState('');
  const [expansion, setExpansion] = useState('');
  const [wholeUtterance, setWholeUtterance] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [editIndex, setEditIndex] = useState<number | null>(null);
  const [editTrigger, setEditTrigger] = useState('');
  const [editExpansion, setEditExpansion] = useState('');
  const [editWhole, setEditWhole] = useState(false);
  const [editError, setEditError] = useState<string | null>(null);
  const listRef = useRef<HTMLDivElement>(null);

  const matches = filterSnippets(settings.snippets, query);

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
    if (editIndex === index) cancelEdit();
    else if (editIndex !== null && index < editIndex) setEditIndex(editIndex - 1);
    void save({
      ...settings,
      snippets: settings.snippets.filter((_, i) => i !== index),
    });
  };

  const startEdit = (index: number, snippet: Snippet): void => {
    setEditIndex(index);
    setEditTrigger(snippet.trigger);
    setEditExpansion(snippet.expansion);
    setEditWhole(snippet.wholeUtterance);
    setEditError(null);
  };

  // Leaving edit mode unmounts the focused control; parking focus on the list
  // container keeps keyboard users in the list instead of dropping to <body>.
  const cancelEdit = (): void => {
    setEditIndex(null);
    setEditError(null);
    listRef.current?.focus();
  };

  const saveEdit = (index: number): void => {
    const snippet = {
      trigger: editTrigger.trim(),
      expansion: editExpansion,
      wholeUtterance: editWhole,
    };
    const others = settings.snippets.filter((_, i) => i !== index);
    const problem = validateSnippet(snippet, others);
    if (problem) {
      setEditError(problem);
      return;
    }
    const next = settings.snippets.map((existing, i) => (i === index ? snippet : existing));
    setEditIndex(null);
    setEditError(null);
    void save({ ...settings, snippets: next });
    listRef.current?.focus();
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
            aria-label="Trigger phrase"
            value={trigger}
            onChange={(e) => {
              setTrigger(e.target.value);
            }}
          />
          <textarea
            className="snippet-expansion"
            placeholder="Insert this… (e.g. “me@example.com”)"
            aria-label="Expansion"
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

        {settings.snippets.length > 0 && (
          <input
            type="search"
            className="dict-search"
            placeholder="Search snippets"
            aria-label="Search snippets"
            value={query}
            onChange={(e) => {
              setQuery(e.target.value);
            }}
          />
        )}

        {settings.snippets.length === 0 ? (
          <p className="row-hint">No snippets yet.</p>
        ) : matches.length === 0 ? (
          <p className="row-hint">No matches.</p>
        ) : (
          <div className="dict-list" ref={listRef} tabIndex={-1}>
            {matches.map(({ snippet, index }) =>
              editIndex === index ? (
                <div key={`edit-${index}`} className="snippet-row snippet-row-editing">
                  {/* Entering edit mode unmounts the focused Edit button. */}
                  <div className="snippet-row-text">
                    <input
                      type="text"
                      aria-label="Trigger phrase"
                      autoFocus
                      value={editTrigger}
                      onChange={(e) => {
                        setEditTrigger(e.target.value);
                      }}
                    />
                    <textarea
                      className="snippet-expansion"
                      aria-label="Expansion"
                      rows={3}
                      value={editExpansion}
                      onChange={(e) => {
                        setEditExpansion(e.target.value);
                      }}
                    />
                    <div className="snippet-scope">
                      <Toggle
                        checked={editWhole}
                        onChange={setEditWhole}
                        label="Expand only when spoken alone"
                      />
                      <span>Only when spoken alone</span>
                    </div>
                  </div>
                  <div className="snippet-row-actions">
                    <button
                      className="btn"
                      onClick={() => {
                        saveEdit(index);
                      }}
                    >
                      Save
                    </button>
                    <button className="btn btn-quiet" onClick={cancelEdit}>
                      Cancel
                    </button>
                  </div>
                </div>
              ) : (
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
                      startEdit(index, snippet);
                    }}
                  >
                    Edit
                  </button>
                  <button
                    className="btn btn-quiet"
                    onClick={() => {
                      removeAt(index);
                    }}
                  >
                    Remove
                  </button>
                </div>
              ),
            )}
          </div>
        )}
        {editError && <p className="form-error">{editError}</p>}
      </section>
    </div>
  );
}
