import { type JSX } from 'react';
import type { Transform } from '@openflow/core';
import type { SettingsApi } from '../hooks.js';
import { HotkeyRecorder } from './HotkeyRecorder.js';
import { Row } from './Row.js';

/** One-click starting points; the user assigns a hotkey afterwards. */
const TRANSFORM_TEMPLATES: { name: string; instruction: string }[] = [
  {
    name: 'Concise',
    instruction:
      'Tighten the wording so it is as concise as possible. Keep the meaning, tone, and language. Do not add new information.',
  },
  {
    name: 'Bullet points',
    instruction:
      'Restructure the text into short, scannable bullet points. Keep the meaning and language; do not invent details.',
  },
  {
    name: 'Friendlier',
    instruction:
      'Rewrite in a warmer, friendlier tone. Keep the meaning and language; do not add new facts.',
  },
  {
    name: 'Formal',
    instruction:
      'Rewrite in a polished, professional tone. Keep the meaning and language; do not add new information.',
  },
];

/**
 * The Polish card: the built-in fix-grammar shortcut plus custom prompts
 * (transforms), each mapped to its own shortcut; all apply to the current
 * selection via the active AI profile.
 */
export function PolishShortcuts({ api }: { api: SettingsApi }): JSX.Element {
  const { settings, update } = api;

  const addTransform = (seed: { name: string; instruction: string }): void => {
    const transform: Transform = {
      id: crypto.randomUUID(),
      name: seed.name,
      instruction: seed.instruction,
      hotkey: '',
    };
    void update({ transforms: [...settings.transforms, transform] });
  };

  const patchTransform = (id: string, patch: Partial<Transform>): void => {
    void update({
      transforms: settings.transforms.map((t) => (t.id === id ? { ...t, ...patch } : t)),
    });
  };

  const removeTransform = (id: string): void => {
    void update({ transforms: settings.transforms.filter((t) => t.id !== id) });
  };

  return (
    <section className="card">
      <h2>Polish</h2>
      <p className="row-hint">
        Prompts mapped to shortcuts. Select text in any app and press a shortcut to rewrite it in
        place — no voice. Needs an AI profile.
      </p>

      <Row
        title="Polish"
        hint="Fixes grammar, spelling, and clarity. Keeps your meaning, tone, and language."
      >
        <HotkeyRecorder
          value={settings.polishHotkey}
          label="Polish"
          onChange={(accelerator) => void update({ polishHotkey: accelerator })}
        />
      </Row>

      {settings.transforms.length > 0 && (
        <div className="transform-list">
          {settings.transforms.map((t) => (
            <div key={t.id} className="transform-card">
              <div className="transform-head">
                <input
                  type="text"
                  className="transform-name"
                  value={t.name}
                  maxLength={40}
                  placeholder="Name"
                  aria-label="Transform name"
                  onChange={(e) => {
                    patchTransform(t.id, { name: e.target.value });
                  }}
                />
                <HotkeyRecorder
                  value={t.hotkey}
                  label={t.name || 'Transform'}
                  onChange={(hotkey) => {
                    patchTransform(t.id, { hotkey });
                  }}
                />
                <button
                  className="btn btn-quiet"
                  onClick={() => {
                    removeTransform(t.id);
                  }}
                >
                  Remove
                </button>
              </div>
              <textarea
                className="transform-instruction"
                rows={2}
                maxLength={2000}
                value={t.instruction}
                placeholder="How should this rewrite the selection? (leave empty to act like Polish)"
                aria-label="Transform instruction"
                onChange={(e) => {
                  patchTransform(t.id, { instruction: e.target.value });
                }}
              />
              {t.hotkey.trim() === '' && (
                <p className="row-hint">Assign a shortcut to use this prompt.</p>
              )}
            </div>
          ))}
        </div>
      )}

      <div className="row-actions transform-templates">
        {TRANSFORM_TEMPLATES.map((tpl) => (
          <button
            key={tpl.name}
            className="btn btn-quiet"
            onClick={() => {
              addTransform(tpl);
            }}
          >
            + {tpl.name}
          </button>
        ))}
        <button
          className="btn"
          onClick={() => {
            addTransform({ name: 'New transform', instruction: '' });
          }}
        >
          Create your own
        </button>
      </div>
    </section>
  );
}
