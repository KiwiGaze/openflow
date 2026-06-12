import type { JSX } from 'react';
import type { PolishRules, Transform } from '@velata/core';
import type { SettingsApi } from '../hooks.js';
import { HotkeyRecorder } from '../components/HotkeyRecorder.js';
import { Row } from '../components/Row.js';
import { Toggle } from '../components/Toggle.js';

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

/** The four Polish rule toggles, in the order they read on the card. */
const POLISH_RULES: { key: keyof PolishRules; label: string }[] = [
  { key: 'concise', label: 'Make it more concise' },
  { key: 'clarity', label: 'Reword for clarity' },
  { key: 'structure', label: 'Add structure for readability' },
  { key: 'tone', label: 'Keep your tone' },
];

/**
 * Transforms page: the selection-rewrite tools. The built-in Polish (its
 * hotkey + rule toggles), the built-in Prompt Engineer, and the user's own
 * transforms. Every one applies to the current selection via the active AI
 * profile — no voice.
 */
export function TransformsTab({ api }: { api: SettingsApi }): JSX.Element {
  const { settings, update } = api;
  const builtIns = settings.transforms.filter((t) => t.builtIn);
  const customs = settings.transforms.filter((t) => !t.builtIn);

  const patchTransform = (id: string, patch: Partial<Transform>): void => {
    void update({
      transforms: settings.transforms.map((t) => (t.id === id ? { ...t, ...patch } : t)),
    });
  };

  const addTransform = (seed: { name: string; instruction: string }): void => {
    const transform: Transform = {
      id: crypto.randomUUID(),
      name: seed.name,
      instruction: seed.instruction,
      hotkey: '',
      builtIn: false,
    };
    void update({ transforms: [...settings.transforms, transform] });
  };

  const removeTransform = (id: string): void => {
    void update({ transforms: settings.transforms.filter((t) => t.id !== id) });
  };

  const setRule = (key: keyof PolishRules, value: boolean): void => {
    void update({ polishRules: { ...settings.polishRules, [key]: value } });
  };

  return (
    <div className="tab-body">
      <section className="card">
        <h2>Polish</h2>
        <p className="row-hint">
          Rewrites the selected text. Fix grammar and spelling, plus the rules you enable. Needs an
          AI profile.
        </p>
        <Row title="Polish shortcut" hint="Select text in any app, then press it to rewrite.">
          <HotkeyRecorder
            value={settings.polishHotkey}
            label="Polish"
            onChange={(accelerator) => void update({ polishHotkey: accelerator })}
          />
        </Row>
        {POLISH_RULES.map((rule) => (
          <Row key={rule.key} title={rule.label}>
            <Toggle
              checked={settings.polishRules[rule.key]}
              onChange={(checked) => {
                setRule(rule.key, checked);
              }}
              label={rule.label}
            />
          </Row>
        ))}
      </section>

      {builtIns.map((t) => (
        <section key={t.id} className="card">
          <h2>
            {t.name} <span className="badge badge-muted">Built in</span>
          </h2>
          <p className="row-hint">
            Rewrites the selected text as a clear prompt for an AI model. Edit the instruction or
            change its shortcut; it can&apos;t be deleted.
          </p>
          <Row title="Shortcut" hint="Select text in any app, then press it.">
            <HotkeyRecorder
              value={t.hotkey}
              label={t.name}
              onChange={(hotkey) => {
                patchTransform(t.id, { hotkey });
              }}
            />
          </Row>
          <textarea
            className="transform-instruction"
            rows={3}
            maxLength={2000}
            value={t.instruction}
            aria-label={`${t.name} instruction`}
            onChange={(e) => {
              patchTransform(t.id, { instruction: e.target.value });
            }}
          />
        </section>
      ))}

      <section className="card">
        <h2>Your transforms</h2>
        <p className="row-hint">
          Saved rewrite prompts, each mapped to its own shortcut. Leave the instruction empty to act
          like Polish.
        </p>

        {customs.length > 0 && (
          <div className="transform-list">
            {customs.map((t) => (
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
            Add transform
          </button>
        </div>
      </section>
    </div>
  );
}
