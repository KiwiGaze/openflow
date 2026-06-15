import { useState, type JSX } from 'react';
import type { SettingsApi } from '../hooks.js';
import { DictionaryTab } from './DictionaryTab.js';
import { SnippetsTab } from './SnippetsTab.js';

type Section = 'dictionary' | 'snippets';

/**
 * Library groups the two saved-text editors — the personal Dictionary and
 * Snippets — under one sidebar entry, switched by a segmented control. Each
 * section reuses its existing editor as-is.
 */
export function LibraryTab({ api }: { api: SettingsApi }): JSX.Element {
  const [section, setSection] = useState<Section>('dictionary');

  return (
    <div className="tab-body">
      <h1>Library</h1>
      <div className="library-switch" role="group" aria-label="Library section">
        <button
          type="button"
          className={`chip ${section === 'dictionary' ? 'chip-active' : ''}`}
          aria-pressed={section === 'dictionary'}
          onClick={() => {
            setSection('dictionary');
          }}
        >
          Dictionary
        </button>
        <button
          type="button"
          className={`chip ${section === 'snippets' ? 'chip-active' : ''}`}
          aria-pressed={section === 'snippets'}
          onClick={() => {
            setSection('snippets');
          }}
        >
          Snippets
        </button>
      </div>
      {section === 'dictionary' ? <DictionaryTab api={api} /> : <SnippetsTab api={api} />}
    </div>
  );
}
