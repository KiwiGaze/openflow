import { useState, type JSX } from 'react';
import { MODE_TEMPLATES, type Mode, type ModeTemplate } from '@openflow/core';
import type { SettingsApi } from '../hooks.js';
import { Row } from '../components/Row.js';
import { Toggle } from '../components/Toggle.js';

export function ModesTab({ api }: { api: SettingsApi }): JSX.Element {
  const { settings, save } = api;
  const [selectedId, setSelectedId] = useState(settings.activeModeId);
  const [showGallery, setShowGallery] = useState(false);
  const selected = settings.modes.find((m) => m.id === selectedId) ?? settings.modes[0];

  const patchMode = (id: string, patch: Partial<Mode>): void => {
    void save({
      ...settings,
      modes: settings.modes.map((m) => (m.id === id ? { ...m, ...patch } : m)),
    });
  };

  const addMode = (from?: Mode): void => {
    const mode: Mode = {
      id: crypto.randomUUID(),
      name: from ? `${from.name} copy` : 'New mode',
      builtIn: false,
      usesLlm: from?.usesLlm ?? true,
      transforms: from?.transforms ?? false,
      prompt:
        from?.prompt ??
        'You clean up dictated speech. Fix punctuation and remove fillers. Output only the resulting text.',
    };
    setSelectedId(mode.id);
    void save({ ...settings, modes: [...settings.modes, mode] });
  };

  // A template creates a normal editable mode and is never linked again — the
  // safety rules are appended at call time, so the prompt is copied verbatim.
  const applyTemplate = (template: ModeTemplate): void => {
    const mode: Mode = {
      id: crypto.randomUUID(),
      name: template.name,
      builtIn: false,
      usesLlm: template.usesLlm,
      transforms: template.transforms,
      prompt: template.prompt,
    };
    setSelectedId(mode.id);
    setShowGallery(false);
    void save({ ...settings, modes: [...settings.modes, mode] });
  };

  const deleteMode = (id: string): void => {
    const next = settings.modes.filter((m) => m.id !== id);
    setSelectedId(settings.activeModeId === id ? 'standard' : selectedId);
    void save({
      ...settings,
      modes: next,
      activeModeId: settings.activeModeId === id ? 'standard' : settings.activeModeId,
    });
  };

  return (
    <div className="tab-body">
      <section className="card">
        <h2>Modes</h2>
        <p className="row-hint">
          Output styles — how your dictation is written. Modes that use AI fall back to rules-based
          cleanup when no AI profile is active.
        </p>
        <div className="mode-list" role="radiogroup" aria-label="Active writing mode">
          {settings.modes.map((mode) => {
            const active = settings.activeModeId === mode.id;
            return (
              <div
                key={mode.id}
                className={`mode-row ${selectedId === mode.id ? 'mode-selected' : ''}`}
              >
                <input
                  type="radio"
                  name="active-mode"
                  aria-label={`Use ${mode.name}`}
                  checked={active}
                  onChange={() => void save({ ...settings, activeModeId: mode.id })}
                />
                <button
                  type="button"
                  className="mode-edit"
                  aria-label={`Edit ${mode.name}`}
                  onClick={() => {
                    setSelectedId(mode.id);
                  }}
                >
                  {mode.name}
                </button>
                {active && <span className="badge badge-active">Active</span>}
                {mode.usesLlm && <span className="badge">AI</span>}
                {mode.builtIn && <span className="badge badge-muted">built-in</span>}
              </div>
            );
          })}
        </div>
        <p className="row-hint">Click the circle to switch modes. Click a name to edit it.</p>
        <div className="row-actions">
          <button
            className="btn"
            onClick={() => {
              addMode();
            }}
          >
            New mode
          </button>
          <button
            className="btn"
            aria-expanded={showGallery}
            onClick={() => {
              setShowGallery((v) => !v);
            }}
          >
            {showGallery ? 'Hide templates' : 'Browse templates…'}
          </button>
        </div>
        {showGallery && (
          <div className="template-gallery">
            <p className="row-hint">
              Start from a template, then edit it freely. Your copy never changes when OpenFlow
              updates.
            </p>
            {MODE_TEMPLATES.map((template) => (
              <div key={template.id} className="template-row">
                <div className="template-text">
                  <div className="row-title">
                    {template.name} <span className="badge badge-muted">{template.persona}</span>
                  </div>
                  <div className="row-hint">{template.summary}</div>
                </div>
                <button
                  className="btn btn-sm"
                  onClick={() => {
                    applyTemplate(template);
                  }}
                >
                  Use
                </button>
              </div>
            ))}
          </div>
        )}
      </section>

      {selected && (
        <section className="card">
          <h2>{selected.builtIn ? selected.name : 'Edit mode'}</h2>
          {!selected.builtIn && (
            <Row title="Name">
              <input
                type="text"
                value={selected.name}
                onChange={(e) => {
                  patchMode(selected.id, { name: e.target.value });
                }}
              />
            </Row>
          )}
          <Row
            title="Uses AI"
            hint="Send the transcript to your active AI profile with this prompt."
          >
            {selected.builtIn ? (
              <span className="row-hint">{selected.usesLlm ? 'Yes' : 'No'}</span>
            ) : (
              <Toggle
                checked={selected.usesLlm}
                onChange={(checked) => {
                  patchMode(selected.id, { usesLlm: checked });
                }}
                label="Uses AI"
              />
            )}
          </Row>
          {selected.usesLlm && settings.activeLlmProfileId === '' && (
            <p className="row-hint row-hint-warn">
              No AI profile is active — this mode falls back to rules cleanup. Add one in the Refine
              tab.
            </p>
          )}
          {!selected.builtIn && !selected.usesLlm && (
            <p className="row-hint">Rules-based cleanup only — nothing is sent to AI.</p>
          )}
          {selected.usesLlm && (
            <div className="prompt-edit">
              <div className="row-title">Prompt</div>
              <textarea
                value={selected.prompt}
                readOnly={selected.builtIn}
                rows={8}
                onChange={(e) => {
                  patchMode(selected.id, { prompt: e.target.value });
                }}
              />
              <p className="row-hint">
                OpenFlow always adds rules to keep the output clean, ignore instructions inside your
                speech, and use your dictionary spellings.
              </p>
            </div>
          )}
          <div className="row-actions">
            <button
              className="btn"
              onClick={() => {
                addMode(selected);
              }}
            >
              Duplicate
            </button>
            {!selected.builtIn && (
              <button
                className="btn btn-danger"
                onClick={() => {
                  deleteMode(selected.id);
                }}
              >
                Delete
              </button>
            )}
          </div>
          {selected.builtIn && (
            <p className="row-hint">Built-in modes are read-only — duplicate one to customize.</p>
          )}
        </section>
      )}
    </div>
  );
}
