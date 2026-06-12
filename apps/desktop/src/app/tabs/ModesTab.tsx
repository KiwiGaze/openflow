import { useRef, useState, type JSX } from 'react';
import {
  MODE_TEMPLATES,
  type Mode,
  type ModeTemplate,
  parseModeImport,
  serializeMode,
  slugifyMode,
  todayIso,
  uniqueModeName,
} from '@velata/core';
import type { ModelsApi, SettingsApi } from '../hooks.js';
import { useLlmProfiles } from '../hooks.js';
import { ipc } from '../ipc.js';
import { AppRules } from '../components/AppRules.js';
import { ModeEditor } from '../components/ModeEditor.js';

export function ModesTab({
  api,
  modelsApi,
}: {
  api: SettingsApi;
  modelsApi: ModelsApi;
}): JSX.Element {
  const { settings, save } = api;
  const { profiles } = useLlmProfiles();
  const { models } = modelsApi;
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
    void ipc.exportMode(slugifyMode(mode.name), serializeMode(mode, todayIso()));
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
              Start from a template, then edit it freely. Your copy never changes when Velata
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
        <ModeEditor
          mode={selected}
          settings={settings}
          profiles={profiles}
          models={models}
          onPatch={(patch) => {
            patchMode(selected.id, patch);
          }}
          onDuplicate={() => {
            addMode(selected);
          }}
          onExport={() => {
            exportMode(selected);
          }}
          onDelete={() => {
            deleteMode(selected.id);
          }}
        />
      )}

      <AppRules api={api} />
    </div>
  );
}
