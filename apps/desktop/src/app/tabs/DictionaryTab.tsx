import { useRef, useState, type JSX } from 'react';
import {
  dictionaryToCsv,
  hasDictionaryEntry,
  parseDictionaryCsv,
  validateDictionaryEntry,
  validateVocabularyTerm,
  type DictionaryEntry,
} from '@velata/core';
import { Toggle } from '../components/Toggle.js';
import { entryKind, filterDictionary } from '../dictionaryView.js';
import { useDictionarySuggestions } from '../hooks.js';
import type { SettingsApi } from '../hooks.js';
import { ipc } from '../ipc.js';

/**
 * Validates an edited, already-trimmed entry against the rest of the
 * dictionary. A vocabulary edit (`from === to`) routes to
 * `validateVocabularyTerm` because `validateDictionaryEntry` rejects no-op
 * replacements.
 */
function validateEdit(entry: DictionaryEntry, others: readonly DictionaryEntry[]): string | null {
  return entry.from === entry.to
    ? validateVocabularyTerm(entry.from, others)
    : validateDictionaryEntry(entry, others);
}

export function DictionaryTab({ api }: { api: SettingsApi }): JSX.Element {
  const { settings, save } = api;
  const { suggestions, dismiss, refresh } = useDictionarySuggestions();
  const [query, setQuery] = useState('');
  const [correction, setCorrection] = useState(false);
  const [word, setWord] = useState('');
  const [from, setFrom] = useState('');
  const [to, setTo] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [editIndex, setEditIndex] = useState<number | null>(null);
  const [editFrom, setEditFrom] = useState('');
  const [editTo, setEditTo] = useState('');
  const [editError, setEditError] = useState<string | null>(null);
  const importInputRef = useRef<HTMLInputElement>(null);
  const [importNotice, setImportNotice] = useState<string | null>(null);
  const listRef = useRef<HTMLDivElement>(null);

  const matches = filterDictionary(settings.dictionary, query);

  const exportCsv = (): void => {
    void ipc.exportDictionary(dictionaryToCsv(settings.dictionary));
  };

  const importCsv = async (file: File): Promise<void> => {
    const { entries, skipped } = parseDictionaryCsv(await file.text(), settings.dictionary);
    if (entries.length > 0) {
      await save({ ...settings, dictionary: [...settings.dictionary, ...entries] });
    }
    const noun = (n: number): string => (n === 1 ? 'entry' : 'entries');
    setImportNotice(
      skipped === 0
        ? `Imported ${entries.length} ${noun(entries.length)}.`
        : `Imported ${entries.length} ${noun(entries.length)}. ${skipped} row${
            skipped === 1 ? '' : 's'
          } skipped (invalid or duplicate).`,
    );
  };

  const add = (): void => {
    if (correction) {
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
      return;
    }
    const term = word.trim();
    const problem = validateVocabularyTerm(term, settings.dictionary);
    if (problem) {
      setError(problem);
      return;
    }
    setError(null);
    setWord('');
    void save({ ...settings, dictionary: [...settings.dictionary, { from: term, to: term }] });
  };

  const removeAt = (index: number): void => {
    if (editIndex === index) cancelEdit();
    else if (editIndex !== null && index < editIndex) setEditIndex(editIndex - 1);
    void save({
      ...settings,
      dictionary: settings.dictionary.filter((_, i) => i !== index),
    });
  };

  const startEdit = (index: number, entry: DictionaryEntry): void => {
    setEditIndex(index);
    setEditFrom(entry.from);
    setEditTo(entry.to);
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
    const entry = { from: editFrom.trim(), to: editTo.trim() };
    const others = settings.dictionary.filter((_, i) => i !== index);
    const problem = validateEdit(entry, others);
    if (problem) {
      setEditError(problem);
      return;
    }
    const next = settings.dictionary.map((existing, i) => (i === index ? entry : existing));
    setEditIndex(null);
    setEditError(null);
    void save({ ...settings, dictionary: next });
    listRef.current?.focus();
  };

  // A suggested term is a vocabulary hint: keep this spelling (from === to, so
  // validateDictionaryEntry — which rejects no-op replacements — does not apply
  // here). Suggestions already exclude dictionary terms, but the list can lag a
  // manual add, so guard against creating a duplicate entry.
  const accept = (term: string): void => {
    const trimmed = term.trim();
    if (hasDictionaryEntry(trimmed, settings.dictionary)) {
      refresh();
      return;
    }
    void save({
      ...settings,
      dictionary: [...settings.dictionary, { from: trimmed, to: trimmed }],
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
          {correction ? (
            <>
              <input
                type="text"
                placeholder="Misspelling (e.g. “open flow”)"
                aria-label="Misspelling"
                value={from}
                onChange={(e) => {
                  setFrom(e.target.value);
                }}
              />
              <span className="dict-arrow">→</span>
              <input
                type="text"
                placeholder="Correct spelling (e.g. “Velata”)"
                aria-label="Correct spelling"
                value={to}
                onChange={(e) => {
                  setTo(e.target.value);
                }}
              />
            </>
          ) : (
            <input
              type="text"
              placeholder="Word or phrase (e.g. “Velata”)"
              aria-label="Word or phrase"
              value={word}
              onChange={(e) => {
                setWord(e.target.value);
              }}
            />
          )}
          <button className="btn" type="submit">
            Add
          </button>
        </form>
        <div className="dict-correction-toggle">
          <Toggle
            checked={correction}
            onChange={(next) => {
              setCorrection(next);
              setError(null);
            }}
            label="Correct a misspelling"
          />
          <span>Correct a misspelling</span>
        </div>
        {error && <p className="form-error">{error}</p>}

        {settings.dictionary.length > 0 && (
          <input
            type="search"
            className="dict-search"
            placeholder="Search entries"
            aria-label="Search dictionary entries"
            value={query}
            onChange={(e) => {
              setQuery(e.target.value);
            }}
          />
        )}

        {settings.dictionary.length === 0 ? (
          <p className="row-hint">
            Nothing here yet. When a name or term gets misheard, add it — e.g. “open flow” →
            “Velata”.
          </p>
        ) : matches.length === 0 ? (
          <p className="row-hint">No matches.</p>
        ) : (
          <div className="dict-list" ref={listRef} tabIndex={-1}>
            {matches.map(({ entry, index }) =>
              editIndex === index ? (
                <div key={`edit-${index}`} className="dict-row dict-row-editing">
                  {/* Entering edit mode unmounts the focused Edit button. */}
                  <input
                    type="text"
                    aria-label="Heard as"
                    autoFocus
                    value={editFrom}
                    onChange={(e) => {
                      setEditFrom(e.target.value);
                    }}
                  />
                  <span className="dict-arrow">→</span>
                  <input
                    type="text"
                    aria-label="Replace with"
                    value={editTo}
                    onChange={(e) => {
                      setEditTo(e.target.value);
                    }}
                  />
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
              ) : (
                <div key={`${entry.from}-${index}`} className="dict-row">
                  {entryKind(entry) === 'vocabulary' ? (
                    <>
                      <span className="dict-to">{entry.to}</span>
                      <span className="badge badge-muted">Kept as-is</span>
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
                      startEdit(index, entry);
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

        <div className="row-actions">
          <button
            className="btn btn-quiet"
            disabled={settings.dictionary.length === 0}
            onClick={exportCsv}
          >
            Export CSV
          </button>
          <button className="btn btn-quiet" onClick={() => importInputRef.current?.click()}>
            Import CSV
          </button>
          <input
            ref={importInputRef}
            type="file"
            accept=".csv,text/csv"
            hidden
            onChange={(e) => {
              const file = e.target.files?.[0];
              e.target.value = '';
              if (file) void importCsv(file);
            }}
          />
        </div>
        {importNotice && <p className="row-hint">{importNotice}</p>}
      </section>
    </div>
  );
}
