import type { JSX } from 'react';
import type { CleanupLevel } from '@velata/core';
import type { SettingsApi } from '../hooks.js';
import { AppRules } from '../components/AppRules.js';

const LEVELS: { value: CleanupLevel; label: string }[] = [
  { value: 'off', label: 'Off' },
  { value: 'rules', label: 'Rules' },
  { value: 'ai', label: 'AI polish' },
];

const DESCRIPTIONS: Record<CleanupLevel, string> = {
  off: 'Insert exactly what you said.',
  rules: 'Tidy fillers, casing, and punctuation on this Mac.',
  ai: 'Let the mode decide. AI polish applies when the mode uses it.',
};

export function StyleTab({ api }: { api: SettingsApi }): JSX.Element {
  const level = api.settings.autoCleanupLevel;

  return (
    <div className="tab-body">
      <section className="card">
        <h2>Auto cleanup</h2>
        <div className="dict-views" role="group" aria-label="Auto cleanup level">
          {LEVELS.map((l) => (
            <button
              key={l.value}
              type="button"
              className={`chip ${level === l.value ? 'chip-active' : ''}`}
              aria-pressed={level === l.value}
              onClick={() => {
                void api.update({ autoCleanupLevel: l.value });
              }}
            >
              {l.label}
            </button>
          ))}
        </div>
        <p className="row-hint">{DESCRIPTIONS[level]}</p>
      </section>

      <AppRules api={api} />
    </div>
  );
}
