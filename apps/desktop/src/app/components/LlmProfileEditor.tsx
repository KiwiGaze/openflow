import { useState, type JSX } from 'react';
import {
  clampTimeoutSecs,
  isLocalEndpoint,
  isValidBaseUrl,
  LLM_PRESETS,
  type LlmProfile,
  type LlmTestResult,
  normalizeBaseUrl,
  presetForProfile,
} from '@openflow/core';
import { ipc } from '../ipc.js';
import { Row } from './Row.js';

// Only a change that could alter connectivity invalidates a test result;
// editing the name or timeout keeps the green check (UX-33).
const CONNECTIVITY_KEYS = ['baseUrl', 'apiKey', 'model'] as const;

/**
 * The editor card for one LLM profile. Extracted at the seam documented in
 * monorepo-conventions §"When to split". Render with `key={profile.id}`:
 * test results and Ollama listings belong to one profile, and the remount
 * drops them when the selection switches.
 */
export function LlmProfileEditor({
  profile,
  onSave,
  onDelete,
}: {
  profile: LlmProfile;
  onSave: (profile: LlmProfile) => Promise<void>;
  onDelete: () => void;
}): JSX.Element {
  const [testResult, setTestResult] = useState<LlmTestResult | null>(null);
  const [testing, setTesting] = useState(false);
  const [ollamaModels, setOllamaModels] = useState<string[] | null>(null);
  const [listError, setListError] = useState<string | null>(null);

  const currentPreset = presetForProfile(profile.presetId, profile.provider);
  const urlInvalid = !isValidBaseUrl(profile.baseUrl);

  const patch = (next: Partial<LlmProfile>): void => {
    if (CONNECTIVITY_KEYS.some((k) => k in next)) setTestResult(null);
    void onSave({ ...profile, ...next });
  };

  // A preset prefills the connection fields but never locks them; `custom`
  // keeps whatever is there so a curated URL survives the switch.
  const applyPreset = (presetId: string): void => {
    const preset = LLM_PRESETS.find((p) => p.id === presetId);
    if (!preset) return;
    setTestResult(null);
    setOllamaModels(null);
    setListError(null);
    const next: LlmProfile = { ...profile, presetId, provider: preset.kind };
    if (preset.id !== 'custom') {
      next.baseUrl = preset.baseUrl;
      next.model = preset.modelSuggestion;
    }
    if (!preset.needsKey) next.apiKey = '';
    void onSave(next);
  };

  const runTest = async (): Promise<void> => {
    setTesting(true);
    setTestResult(null);
    try {
      setTestResult(await ipc.testLlm(profile));
    } catch (err) {
      setTestResult({ ok: false, message: String(err) });
    } finally {
      setTesting(false);
    }
  };

  const browseOllama = async (): Promise<void> => {
    setListError(null);
    try {
      setOllamaModels(await ipc.listOllamaModels(normalizeBaseUrl(profile.baseUrl)));
    } catch (err) {
      setOllamaModels(null);
      setListError(String(err));
    }
  };

  return (
    <section className="card">
      <h2>Edit profile</h2>
      <Row title="Name">
        <input
          type="text"
          value={profile.name}
          onChange={(e) => {
            patch({ name: e.target.value });
          }}
        />
      </Row>
      <Row title="Provider" hint={currentPreset.caveat}>
        <select
          value={currentPreset.id}
          onChange={(e) => {
            applyPreset(e.target.value);
          }}
        >
          {LLM_PRESETS.map((preset) => (
            <option key={preset.id} value={preset.id}>
              {preset.displayName}
            </option>
          ))}
        </select>
      </Row>
      <Row
        title="Base URL"
        hint={
          profile.provider === 'ollama'
            ? 'Where Ollama is listening.'
            : 'Endpoint root, e.g. https://api.openai.com/v1 or https://api.groq.com/openai/v1.'
        }
      >
        <input
          type="text"
          className={urlInvalid ? 'input-invalid' : ''}
          value={profile.baseUrl}
          onChange={(e) => {
            patch({ baseUrl: e.target.value });
          }}
        />
      </Row>
      {currentPreset.needsKey && (
        <Row title="API key" hint="Stored in the profile file. Sent only to the base URL above.">
          <input
            type="password"
            value={profile.apiKey}
            autoComplete="off"
            onChange={(e) => {
              patch({ apiKey: e.target.value });
            }}
          />
        </Row>
      )}
      <Row title="Model">
        <div className="stack">
          <input
            type="text"
            value={profile.model}
            placeholder={currentPreset.modelSuggestion}
            onChange={(e) => {
              patch({ model: e.target.value });
            }}
          />
          {profile.provider === 'ollama' && (
            <button className="btn btn-quiet" onClick={() => void browseOllama()}>
              List installed models
            </button>
          )}
        </div>
      </Row>
      {ollamaModels && (
        <div className="ollama-models">
          {ollamaModels.length === 0 ? (
            <p className="row-hint">
              No models installed. Run e.g. <code>ollama pull qwen2.5:3b</code>.
            </p>
          ) : (
            ollamaModels.map((name) => (
              <button
                key={name}
                className={`chip ${profile.model === name ? 'chip-active' : ''}`}
                onClick={() => {
                  patch({ model: name });
                }}
              >
                {name}
              </button>
            ))
          )}
        </div>
      )}
      {listError && <p className="form-error">{listError}</p>}
      <Row title="Timeout" hint="Seconds to wait for a response.">
        <input
          type="number"
          min={5}
          max={300}
          value={profile.timeoutSecs}
          onChange={(e) => {
            patch({ timeoutSecs: clampTimeoutSecs(e.target.value) });
          }}
        />
      </Row>
      <div className="row-actions">
        <button className="btn" disabled={testing || urlInvalid} onClick={() => void runTest()}>
          {testing ? 'Testing…' : 'Test connection'}
        </button>
        {testResult && (
          <span className={testResult.ok ? 'badge badge-ok' : 'form-error'}>
            {testResult.message}
          </span>
        )}
        <button className="btn btn-danger" onClick={onDelete}>
          Delete
        </button>
      </div>
      {!isLocalEndpoint(profile.baseUrl) && (
        <p className="privacy-note">
          Heads up: with a cloud profile, refined text (never audio) is sent to the endpoint above.
        </p>
      )}
    </section>
  );
}
