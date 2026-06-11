import { useRef, useState, type JSX } from 'react';
import {
  LANGUAGES,
  languageLabel,
  MODE_TEMPLATES,
  type Mode,
  type ModeTemplate,
  parseModeImport,
  serializeMode,
  slugifyMode,
  uniqueModeName,
} from '@openflow/core';
import type { SettingsApi } from '../hooks.js';
import { useLlmProfiles, useModels } from '../hooks.js';
import { ipc } from '../ipc.js';
import { AppRules } from '../components/AppRules.js';
import { HotkeyRecorder } from '../components/HotkeyRecorder.js';
import { Row } from '../components/Row.js';
import { Toggle } from '../components/Toggle.js';

/** A messy dictation for Preview: fillers, a self-correction, a name, a number. */
const PREVIEW_SAMPLE =
  'um so yesterday I I shipped the the login fix for sarah and uh closed three tickets';

export function ModesTab({ api }: { api: SettingsApi }): JSX.Element {
  const { settings, save } = api;
  const { profiles } = useLlmProfiles();
  const { models } = useModels();
  const [selectedId, setSelectedId] = useState(settings.activeModeId);
  const [showGallery, setShowGallery] = useState(false);
  const selected = settings.modes.find((m) => m.id === selectedId) ?? settings.modes[0];

  // Labels for the "Inherit — currently …" override rows (07 §7).
  const inheritProfile =
    profiles.find((p) => p.id === settings.activeLlmProfileId)?.name ?? 'No AI';
  const inheritModel =
    models.find((m) => m.id === settings.sttModelId)?.displayName ?? settings.sttModelId;
  const inheritLanguage = languageLabel(settings.language);

  // An empty select value means "inherit" → store null.
  const orNull = (value: string): string | null => (value === '' ? null : value);

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
      aiProfileId: from?.aiProfileId ?? null,
      sttModelId: from?.sttModelId ?? null,
      language: from?.language ?? null,
      // A copy never inherits the hotkey — two modes can't share one.
      hotkey: null,
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
      aiProfileId: null,
      sttModelId: null,
      language: null,
      hotkey: null,
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

  const importInputRef = useRef<HTMLInputElement>(null);
  const [importNotice, setImportNotice] = useState<string | null>(null);

  const exportMode = (mode: Mode): void => {
    const today = new Date().toISOString().slice(0, 10);
    void ipc.exportMode(slugifyMode(mode.name), serializeMode(mode, today));
  };

  // Each imported file gets a fresh id and a Finder-style unique name; an
  // import can never overwrite an existing mode (06 §4).
  const importFiles = async (files: FileList): Promise<void> => {
    const created: Mode[] = [];
    let skipped = 0;
    for (const file of Array.from(files)) {
      const result = parseModeImport(await file.text());
      if (!result.ok) {
        skipped += 1;
        continue;
      }
      const taken = [...settings.modes, ...created].map((m) => m.name);
      created.push({
        ...result.mode,
        id: crypto.randomUUID(),
        name: uniqueModeName(result.mode.name, taken),
      });
    }
    if (created.length > 0) {
      const last = created[created.length - 1];
      if (last) setSelectedId(last.id);
      await save({ ...settings, modes: [...settings.modes, ...created] });
    }
    const added = created.length;
    setImportNotice(
      skipped === 0
        ? `Added ${added} mode${added === 1 ? '' : 's'}.`
        : `Added ${added} mode${added === 1 ? '' : 's'}. ${skipped} file${
            skipped === 1 ? ' was' : 's were'
          } skipped (not a valid mode).`,
    );
  };

  const [previewResult, setPreviewResult] = useState<string | null>(null);
  const [previewError, setPreviewError] = useState<string | null>(null);
  const [previewing, setPreviewing] = useState(false);

  const runPreview = async (): Promise<void> => {
    if (!selected) return;
    setPreviewing(true);
    setPreviewError(null);
    setPreviewResult(null);
    try {
      setPreviewResult(
        await ipc.testMode(
          selected.prompt,
          PREVIEW_SAMPLE,
          selected.transforms,
          selected.aiProfileId,
        ),
      );
    } catch (err) {
      setPreviewError(String(err));
    } finally {
      setPreviewing(false);
    }
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
          <button className="btn btn-quiet" onClick={() => importInputRef.current?.click()}>
            Import mode…
          </button>
          <input
            ref={importInputRef}
            type="file"
            accept=".json,application/json"
            multiple
            hidden
            onChange={(e) => {
              const { files } = e.target;
              e.target.value = '';
              if (files && files.length > 0) void importFiles(files);
            }}
          />
        </div>
        {importNotice && <p className="row-hint">{importNotice}</p>}
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
          {selected.usesLlm && (
            <div className="prompt-edit">
              <div className="row-actions">
                <button className="btn" disabled={previewing} onClick={() => void runPreview()}>
                  {previewing ? 'Previewing…' : 'Preview'}
                </button>
                <span className="row-hint">sample: “{PREVIEW_SAMPLE}”</span>
              </div>
              {previewResult !== null && (
                <>
                  <p className="result-text">{previewResult}</p>
                  {settings.activeLlmProfileId === '' && (
                    <p className="row-hint">
                      No AI profile active — showing rules-based cleanup. Add an AI profile to
                      preview this instruction.
                    </p>
                  )}
                </>
              )}
              {previewError && <p className="form-error">{previewError}</p>}
            </div>
          )}
          <details className="advanced">
            <summary>Advanced — AI profile · Speech model · Language</summary>
            {selected.builtIn ? (
              <>
                {selected.usesLlm && (
                  <Row title="AI profile">
                    <span className="row-hint">Inherit — currently {inheritProfile}</span>
                  </Row>
                )}
                <Row title="Speech model">
                  <span className="row-hint">Inherit — currently {inheritModel}</span>
                </Row>
                <Row title="Language">
                  <span className="row-hint">Inherit — currently {inheritLanguage}</span>
                </Row>
                <p className="row-hint">Duplicate this mode to change these.</p>
              </>
            ) : (
              <>
                {selected.usesLlm && (
                  <Row title="AI profile">
                    <select
                      value={selected.aiProfileId ?? ''}
                      onChange={(e) => {
                        patchMode(selected.id, { aiProfileId: orNull(e.target.value) });
                      }}
                    >
                      <option value="">Inherit — currently {inheritProfile}</option>
                      {profiles.map((profile) => (
                        <option key={profile.id} value={profile.id}>
                          {profile.name}
                        </option>
                      ))}
                    </select>
                  </Row>
                )}
                <Row title="Speech model">
                  <select
                    value={selected.sttModelId ?? ''}
                    onChange={(e) => {
                      patchMode(selected.id, { sttModelId: orNull(e.target.value) });
                    }}
                  >
                    <option value="">Inherit — currently {inheritModel}</option>
                    {models
                      .filter((model) => model.installed)
                      .map((model) => (
                        <option key={model.id} value={model.id}>
                          {model.displayName}
                        </option>
                      ))}
                  </select>
                </Row>
                <Row title="Language">
                  <select
                    value={selected.language ?? ''}
                    onChange={(e) => {
                      patchMode(selected.id, { language: orNull(e.target.value) });
                    }}
                  >
                    <option value="">Inherit — currently {inheritLanguage}</option>
                    {LANGUAGES.map(([code, name]) => (
                      <option key={code} value={code}>
                        {name}
                      </option>
                    ))}
                  </select>
                </Row>
                <Row
                  title="Mode hotkey"
                  hint="One-shot: dictates in this mode; the active mode is unchanged."
                >
                  <div className="stack">
                    <HotkeyRecorder
                      value={selected.hotkey ?? ''}
                      label={selected.name}
                      onChange={(accelerator) => {
                        patchMode(selected.id, { hotkey: accelerator });
                      }}
                    />
                    {selected.hotkey && (
                      <button
                        className="btn btn-quiet btn-sm"
                        onClick={() => {
                          patchMode(selected.id, { hotkey: null });
                        }}
                      >
                        Clear
                      </button>
                    )}
                  </div>
                </Row>
              </>
            )}
          </details>
          <div className="row-actions">
            <button
              className="btn"
              onClick={() => {
                addMode(selected);
              }}
            >
              Duplicate
            </button>
            <button
              className="btn"
              onClick={() => {
                exportMode(selected);
              }}
            >
              Export
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

      <AppRules api={api} />
    </div>
  );
}
