import { useState, type JSX } from 'react';
import {
  isLocalEndpoint,
  isValidBaseUrl,
  normalizeBaseUrl,
  type LlmProfile,
  type LlmProviderKind,
  type LlmTestResult,
  type Transform,
} from '@openflow/core';
import type { SettingsApi } from '../hooks.js';
import { useLlmProfiles } from '../hooks.js';
import { ipc } from '../ipc.js';
import { HotkeyRecorder } from '../components/HotkeyRecorder.js';
import { Row } from '../components/Row.js';
import { Toggle } from '../components/Toggle.js';

const PROVIDER_DEFAULTS: Record<LlmProviderKind, Partial<LlmProfile>> = {
  ollama: { baseUrl: 'http://localhost:11434', model: 'qwen2.5:3b', apiKey: '' },
  openaiCompatible: { baseUrl: 'https://api.openai.com/v1', model: 'gpt-4o-mini' },
};

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

export function RefineTab({ api }: { api: SettingsApi }): JSX.Element {
  const { settings, update } = api;
  const { profiles, save, remove } = useLlmProfiles();
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [testResult, setTestResult] = useState<LlmTestResult | null>(null);
  const [testing, setTesting] = useState(false);
  const [ollamaModels, setOllamaModels] = useState<string[] | null>(null);
  const [listError, setListError] = useState<string | null>(null);

  const selected = profiles.find((p) => p.id === selectedId) ?? null;

  // Test results and Ollama listings belong to one profile; drop them on switch.
  const selectProfile = (id: string | null): void => {
    setSelectedId(id);
    setTestResult(null);
    setOllamaModels(null);
    setListError(null);
  };

  const patch = (next: Partial<LlmProfile>): void => {
    if (!selected) return;
    setTestResult(null);
    void save({ ...selected, ...next });
  };

  const switchProvider = (provider: LlmProviderKind): void => {
    if (!selected) return;
    setTestResult(null);
    setOllamaModels(null);
    setListError(null);
    void save({ ...selected, provider, ...PROVIDER_DEFAULTS[provider] });
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
    };
    selectProfile(profile.id);
    void save(profile).then(() => {
      // The first profile is what the user came for: select it for refinement.
      if (settings.activeLlmProfileId === '') void update({ activeLlmProfileId: profile.id });
    });
  };

  const deleteProfile = (id: string): void => {
    if (selectedId === id) selectProfile(null);
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
              role="button"
              tabIndex={0}
              onClick={() => {
                selectProfile(profile.id);
              }}
              onKeyDown={(e) => {
                // Keys on the nested radio bubble up here; leave those alone.
                if (e.target !== e.currentTarget) return;
                if (e.key === 'Enter' || e.key === ' ') {
                  e.preventDefault();
                  selectProfile(profile.id);
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
          <Row title="Provider">
            <select
              value={selected.provider}
              onChange={(e) => {
                switchProvider(e.target.value as LlmProviderKind);
              }}
            >
              <option value="ollama">Ollama (local)</option>
              <option value="openaiCompatible">OpenAI-compatible API (your key)</option>
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
          {selected.provider === 'openaiCompatible' && (
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

      <section className="card">
        <h2>Transforms</h2>
        <p className="row-hint">
          One-tap rewrites for selected text — like Polish, but with your own instruction and
          hotkey. Select text in any app and press the transform&rsquo;s hotkey. Needs an AI
          profile.
        </p>

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
                    onChange={(e) => {
                      patchTransform(t.id, { name: e.target.value });
                    }}
                  />
                  <HotkeyRecorder
                    value={t.hotkey}
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
                  placeholder="How should this rewrite the selected text? (empty acts like Polish)"
                  onChange={(e) => {
                    patchTransform(t.id, { instruction: e.target.value });
                  }}
                />
                {t.hotkey.trim() === '' && (
                  <p className="row-hint">Set a hotkey above to use this transform.</p>
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
    </div>
  );
}
