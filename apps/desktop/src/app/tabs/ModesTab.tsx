import { useState, type JSX } from 'react';
import type { Mode } from '@openflow/core';
import type { SettingsApi } from '../hooks.js';
import { Row } from '../components/Row.js';
import { Toggle } from '../components/Toggle.js';

export function ModesTab({ api }: { api: SettingsApi }): JSX.Element {
  const { settings, save } = api;
  const [selectedId, setSelectedId] = useState(settings.activeModeId);
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
      prompt:
        from?.prompt ??
        'You clean up dictated speech. Fix punctuation and remove fillers. Output only the resulting text.',
    };
    setSelectedId(mode.id);
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
          The active mode shapes how transcripts are written out. Modes that use AI fall back to
          rules-based cleanup when no provider is configured.
        </p>
        <div className="mode-list">
          {settings.modes.map((mode) => (
            <div
              key={mode.id}
              className={`mode-row ${selectedId === mode.id ? 'mode-selected' : ''}`}
              role="button"
              tabIndex={0}
              onClick={() => {
                setSelectedId(mode.id);
              }}
              onKeyDown={(e) => {
                // Keys on the nested radio bubble up here; leave those alone.
                if (e.target !== e.currentTarget) return;
                if (e.key === 'Enter' || e.key === ' ') {
                  e.preventDefault();
                  setSelectedId(mode.id);
                }
              }}
            >
              <label
                onClick={(e) => {
                  e.stopPropagation();
                }}
              >
                <input
                  type="radio"
                  name="active-mode"
                  checked={settings.activeModeId === mode.id}
                  onChange={() => void save({ ...settings, activeModeId: mode.id })}
                />
              </label>
              <span className="row-title">{mode.name}</span>
              {mode.usesLlm && <span className="badge">AI</span>}
              {mode.builtIn && <span className="badge badge-muted">built-in</span>}
            </div>
          ))}
        </div>
        <button
          className="btn"
          onClick={() => {
            addMode();
          }}
        >
          New mode
        </button>
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
          <Row title="Uses AI" hint="Send the transcript to your AI provider with this prompt.">
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
