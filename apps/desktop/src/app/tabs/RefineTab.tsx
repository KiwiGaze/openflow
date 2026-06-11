import { useState, type JSX } from 'react';
import {
  isLocalEndpoint,
  isValidBaseUrl,
  LLM_PRESETS,
  type LlmProfile,
  type LlmTestResult,
  normalizeBaseUrl,
  presetForProfile,
} from '@openflow/core';
import type { SettingsApi } from '../hooks.js';
import { useLlmProfiles } from '../hooks.js';
import { ipc } from '../ipc.js';
import { Row } from '../components/Row.js';
import { Toggle } from '../components/Toggle.js';

export function RefineTab({ api }: { api: SettingsApi }): JSX.Element {
  const { settings, update } = api;
  const { profiles, save, remove } = useLlmProfiles();
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [testResult, setTestResult] = useState<LlmTestResult | null>(null);
  const [testing, setTesting] = useState(false);
  const [ollamaModels, setOllamaModels] = useState<string[] | null>(null);
  const [listError, setListError] = useState<string | null>(null);

  const selected = profiles.find((p) => p.id === selectedId) ?? null;
  const currentPreset = selected ? presetForProfile(selected.presetId, selected.provider) : null;

  // Only a change that could alter connectivity invalidates a test result;
  // editing the name or timeout keeps the green check (UX-33).
  const CONNECTIVITY_KEYS = ['baseUrl', 'apiKey', 'model'] as const;

  const patch = (next: Partial<LlmProfile>): void => {
    if (!selected) return;
    if (CONNECTIVITY_KEYS.some((k) => k in next)) setTestResult(null);
    void save({ ...selected, ...next });
  };

  // A preset prefills the connection fields but never locks them; `custom`
  // keeps whatever is there so a curated URL survives the switch.
  const applyPreset = (presetId: string): void => {
    if (!selected) return;
    const preset = LLM_PRESETS.find((p) => p.id === presetId);
    if (!preset) return;
    setTestResult(null);
    setOllamaModels(null);
    setListError(null);
    const next: LlmProfile = { ...selected, presetId, provider: preset.kind };
    if (preset.id !== 'custom') {
      next.baseUrl = preset.baseUrl;
      next.model = preset.modelSuggestion;
    }
    if (!preset.needsKey) next.apiKey = '';
    void save(next);
  };

  const addProfile = (): void => {
    const profile: LlmProfile = {
      version: 1,
      id: crypto.randomUUID(),
      name: 'New profile',
      provider: 'ollama',
      baseUrl: 'http://localhost:11434',
      apiKey: '',
      model: 'qwen2.5:3b',
      timeoutSecs: 30,
      presetId: 'ollama',
    };
    setSelectedId(profile.id);
    void save(profile).then(() => {
      // The first profile is what the user came for: select it for refinement.
      if (settings.activeLlmProfileId === '') void update({ activeLlmProfileId: profile.id });
    });
  };

  const deleteProfile = (id: string): void => {
    if (selectedId === id) setSelectedId(null);
    // Deleting the active profile clears the selection backend-side.
    void remove(id);
  };

  const runTest = async (): Promise<void> => {
    if (!selected) return;
    setTesting(true);
    setTestResult(null);
    try {
      setTestResult(await ipc.testLlm(selected));
    } catch (err) {
      setTestResult({ ok: false, message: String(err) });
    } finally {
      setTesting(false);
    }
  };

  const browseOllama = async (): Promise<void> => {
    if (!selected) return;
    setListError(null);
    try {
      setOllamaModels(await ipc.listOllamaModels(normalizeBaseUrl(selected.baseUrl)));
    } catch (err) {
      setOllamaModels(null);
      setListError(String(err));
    }
  };

  const urlInvalid = selected !== null && !isValidBaseUrl(selected.baseUrl);

  return (
    <div className="tab-body">
      <section className="card">
        <h2>Refine</h2>
        <Row
          title="Refine dictation with AI"
          hint="Polish transcripts with the active profile after transcribing."
        >
          <Toggle
            checked={settings.refineAfterDictation}
            onChange={(checked) => void update({ refineAfterDictation: checked })}
            label="Refine dictation with AI"
          />
        </Row>
      </section>

      <section className="card">
        <h2>AI profiles</h2>
        {profiles.length === 0 && (
          <p className="row-hint">
            Dictation uses fast rules-based cleanup. Add a profile to enable AI polish and the
            selection shortcuts — nothing leaves your Mac unless you pick a cloud endpoint.
          </p>
        )}
        <div className="mode-list">
          <div className="mode-row">
            <label>
              <input
                type="radio"
                name="active-profile"
                checked={settings.activeLlmProfileId === ''}
                onChange={() => void update({ activeLlmProfileId: '' })}
              />
            </label>
            <span className="row-title">No AI — rules-based cleanup only</span>
          </div>
          {profiles.map((profile) => (
            <div
              key={profile.id}
              className={`mode-row ${selectedId === profile.id ? 'mode-selected' : ''}`}
              onClick={() => {
                setSelectedId(profile.id);
              }}
            >
              <label
                onClick={(e) => {
                  e.stopPropagation();
                }}
              >
                <input
                  type="radio"
                  name="active-profile"
                  checked={settings.activeLlmProfileId === profile.id}
                  onChange={() => void update({ activeLlmProfileId: profile.id })}
                />
              </label>
              <span className="row-title">{profile.name}</span>
              <span className="badge">{isLocalEndpoint(profile.baseUrl) ? 'local' : 'cloud'}</span>
              {profile.model && <span className="badge badge-muted">{profile.model}</span>}
            </div>
          ))}
        </div>
        <div className="row-actions">
          <button className="btn btn-quiet" onClick={() => void ipc.revealLlmProfiles()}>
            Show in Finder
          </button>
          <button className="btn" onClick={addProfile}>
            New profile
          </button>
        </div>
      </section>

      {selected && (
        <section className="card">
          <h2>Edit profile</h2>
          <Row title="Name">
            <input
              type="text"
              value={selected.name}
              onChange={(e) => {
                patch({ name: e.target.value });
              }}
            />
          </Row>
          <Row title="Provider" hint={currentPreset?.caveat ?? ''}>
            <select
              value={currentPreset?.id ?? 'custom'}
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
              selected.provider === 'ollama'
                ? 'Where Ollama is listening.'
                : 'Endpoint root, e.g. https://api.openai.com/v1 or https://api.groq.com/openai/v1.'
            }
          >
            <input
              type="text"
              className={urlInvalid ? 'input-invalid' : ''}
              value={selected.baseUrl}
              onChange={(e) => {
                patch({ baseUrl: e.target.value });
              }}
            />
          </Row>
          {currentPreset?.needsKey && (
            <Row
              title="API key"
              hint="Stored in the profile file. Sent only to the base URL above."
            >
              <input
                type="password"
                value={selected.apiKey}
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
                value={selected.model}
                placeholder={currentPreset?.modelSuggestion ?? ''}
                onChange={(e) => {
                  patch({ model: e.target.value });
                }}
              />
              {selected.provider === 'ollama' && (
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
                    className={`chip ${selected.model === name ? 'chip-active' : ''}`}
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
              value={selected.timeoutSecs}
              onChange={(e) => {
                patch({ timeoutSecs: Math.max(5, Number(e.target.value) || 30) });
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
            <button
              className="btn btn-danger"
              onClick={() => {
                deleteProfile(selected.id);
              }}
            >
              Delete
            </button>
          </div>
          {!isLocalEndpoint(selected.baseUrl) && (
            <p className="privacy-note">
              Heads up: with a cloud profile, refined text (never audio) is sent to the endpoint
              above.
            </p>
          )}
        </section>
      )}
    </div>
  );
}
