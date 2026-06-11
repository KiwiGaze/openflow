import { useState, type JSX } from 'react';
import {
  LANGUAGES,
  languageLabel,
  type LlmProfile,
  type Mode,
  type ModelInfo,
  type Settings,
} from '@openflow/core';
import { ipc } from '../ipc.js';
import { HotkeyRecorder } from './HotkeyRecorder.js';
import { Row } from './Row.js';
import { Toggle } from './Toggle.js';

/** A messy dictation for Preview: fillers, a self-correction, a name, a number. */
const PREVIEW_SAMPLE =
  'um so yesterday I I shipped the the login fix for sarah and uh closed three tickets';

/** An empty select value means "inherit" → store null. */
const orNull = (value: string): string | null => (value === '' ? null : value);

/**
 * The editor card for one mode: name, prompt, preview, advanced overrides,
 * and lifecycle actions. Extracted at the seam documented in
 * monorepo-conventions §"When to split"; list management stays in ModesTab.
 */
export function ModeEditor({
  mode,
  settings,
  profiles,
  models,
  onPatch,
  onDuplicate,
  onExport,
  onDelete,
}: {
  mode: Mode;
  settings: Settings;
  profiles: LlmProfile[];
  models: ModelInfo[];
  onPatch: (patch: Partial<Mode>) => void;
  onDuplicate: () => void;
  onExport: () => void;
  onDelete: () => void;
}): JSX.Element {
  // Labels for the "Inherit — currently …" override rows (07 §7).
  const inheritProfile =
    profiles.find((p) => p.id === settings.activeLlmProfileId)?.name ?? 'No AI';
  const inheritModel =
    models.find((m) => m.id === settings.sttModelId)?.displayName ?? settings.sttModelId;
  const inheritLanguage = languageLabel(settings.language);

  const [previewResult, setPreviewResult] = useState<string | null>(null);
  const [previewError, setPreviewError] = useState<string | null>(null);
  const [previewing, setPreviewing] = useState(false);

  const runPreview = async (): Promise<void> => {
    setPreviewing(true);
    setPreviewError(null);
    setPreviewResult(null);
    try {
      setPreviewResult(
        await ipc.testMode(mode.prompt, PREVIEW_SAMPLE, mode.transforms, mode.aiProfileId),
      );
    } catch (err) {
      setPreviewError(String(err));
    } finally {
      setPreviewing(false);
    }
  };

  return (
    <section className="card">
      <h2>{mode.builtIn ? mode.name : 'Edit mode'}</h2>
      {!mode.builtIn && (
        <Row title="Name">
          <input
            type="text"
            value={mode.name}
            onChange={(e) => {
              onPatch({ name: e.target.value });
            }}
          />
        </Row>
      )}
      <Row title="Uses AI" hint="Send the transcript to your active AI profile with this prompt.">
        {mode.builtIn ? (
          <span className="row-hint">{mode.usesLlm ? 'Yes' : 'No'}</span>
        ) : (
          <Toggle
            checked={mode.usesLlm}
            onChange={(checked) => {
              onPatch({ usesLlm: checked });
            }}
            label="Uses AI"
          />
        )}
      </Row>
      {mode.usesLlm && settings.activeLlmProfileId === '' && (
        <p className="row-hint row-hint-warn">
          No AI profile is active — this mode falls back to rules cleanup. Add one in the Refine
          tab.
        </p>
      )}
      {!mode.builtIn && !mode.usesLlm && (
        <p className="row-hint">Rules-based cleanup only — nothing is sent to AI.</p>
      )}
      {mode.usesLlm && (
        <div className="prompt-edit">
          <div className="row-title">Prompt</div>
          <textarea
            value={mode.prompt}
            readOnly={mode.builtIn}
            rows={8}
            onChange={(e) => {
              onPatch({ prompt: e.target.value });
            }}
          />
          <p className="row-hint">
            OpenFlow always adds rules to keep the output clean, ignore instructions inside your
            speech, and use your dictionary spellings.
          </p>
        </div>
      )}
      {mode.usesLlm && (
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
                  No AI profile active — showing rules-based cleanup. Add an AI profile to preview
                  this instruction.
                </p>
              )}
            </>
          )}
          {previewError && <p className="form-error">{previewError}</p>}
        </div>
      )}
      <details className="advanced">
        <summary>Advanced — AI profile · Speech model · Language</summary>
        {mode.builtIn ? (
          <>
            {mode.usesLlm && (
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
            {mode.usesLlm && (
              <Row title="AI profile">
                <select
                  value={mode.aiProfileId ?? ''}
                  onChange={(e) => {
                    onPatch({ aiProfileId: orNull(e.target.value) });
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
                value={mode.sttModelId ?? ''}
                onChange={(e) => {
                  onPatch({ sttModelId: orNull(e.target.value) });
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
                value={mode.language ?? ''}
                onChange={(e) => {
                  onPatch({ language: orNull(e.target.value) });
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
                  value={mode.hotkey ?? ''}
                  label={mode.name}
                  onChange={(accelerator) => {
                    onPatch({ hotkey: accelerator });
                  }}
                />
                {mode.hotkey && (
                  <button
                    className="btn btn-quiet btn-sm"
                    onClick={() => {
                      onPatch({ hotkey: null });
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
        <button className="btn" onClick={onDuplicate}>
          Duplicate
        </button>
        <button className="btn" onClick={onExport}>
          Export
        </button>
        {!mode.builtIn && (
          <button className="btn btn-danger" onClick={onDelete}>
            Delete
          </button>
        )}
      </div>
      {mode.builtIn && (
        <p className="row-hint">Built-in modes are read-only — duplicate one to customize.</p>
      )}
    </section>
  );
}
