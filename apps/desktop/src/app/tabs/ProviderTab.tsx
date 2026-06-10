import { useState, type JSX } from 'react';
import {
  isValidBaseUrl,
  normalizeBaseUrl,
  type LlmConfig,
  type LlmProviderKind,
  type LlmTestResult,
} from '@openflow/core';
import type { SettingsApi } from '../hooks.js';
import { ipc } from '../ipc.js';
import { Row } from '../components/Row.js';

const PROVIDER_DEFAULTS: Record<LlmProviderKind, Partial<LlmConfig>> = {
  none: {},
  ollama: { baseUrl: 'http://localhost:11434', model: 'qwen2.5:3b', apiKey: '' },
  openaiCompatible: { baseUrl: 'https://api.openai.com/v1', model: 'gpt-4o-mini' },
};

export function ProviderTab({ api }: { api: SettingsApi }): JSX.Element {
  const { settings, update } = api;
  const llm = settings.llm;
  const [testResult, setTestResult] = useState<LlmTestResult | null>(null);
  const [testing, setTesting] = useState(false);
  const [ollamaModels, setOllamaModels] = useState<string[] | null>(null);
  const [listError, setListError] = useState<string | null>(null);

  const patch = (next: Partial<LlmConfig>): void => {
    setTestResult(null);
    void update({ llm: { ...llm, ...next } });
  };

  const switchProvider = (provider: LlmProviderKind): void => {
    setTestResult(null);
    setOllamaModels(null);
    setListError(null);
    void update({ llm: { ...llm, provider, ...PROVIDER_DEFAULTS[provider] } });
  };

  const runTest = async (): Promise<void> => {
    setTesting(true);
    setTestResult(null);
    try {
      setTestResult(await ipc.testLlm(llm));
    } catch (err) {
      setTestResult({ ok: false, message: String(err) });
    } finally {
      setTesting(false);
    }
  };

  const browseOllama = async (): Promise<void> => {
    setListError(null);
    try {
      setOllamaModels(await ipc.listOllamaModels(normalizeBaseUrl(llm.baseUrl)));
    } catch (err) {
      setOllamaModels(null);
      setListError(String(err));
    }
  };

  const urlInvalid = llm.provider !== 'none' && !isValidBaseUrl(llm.baseUrl);

  return (
    <div className="tab-body">
      <section className="card">
        <h2>AI text refinement</h2>
        <p className="row-hint">
          Optional. Without a provider, OpenFlow still dictates using fast rules-based cleanup — and
          nothing ever leaves your Mac.
        </p>
        <Row title="Provider">
          <select
            value={llm.provider}
            onChange={(e) => {
              switchProvider(e.target.value as LlmProviderKind);
            }}
          >
            <option value="none">None — rules-based cleanup only</option>
            <option value="ollama">Ollama (local)</option>
            <option value="openaiCompatible">OpenAI-compatible API (your key)</option>
          </select>
        </Row>

        {llm.provider !== 'none' && (
          <>
            <Row
              title="Base URL"
              hint={
                llm.provider === 'ollama'
                  ? 'Where Ollama is listening.'
                  : 'Endpoint root, e.g. https://api.openai.com/v1 or https://api.groq.com/openai/v1.'
              }
            >
              <input
                type="text"
                className={urlInvalid ? 'input-invalid' : ''}
                value={llm.baseUrl}
                onChange={(e) => {
                  patch({ baseUrl: e.target.value });
                }}
              />
            </Row>
            {llm.provider === 'openaiCompatible' && (
              <Row
                title="API key"
                hint="Stored in OpenFlow’s local settings file. Sent only to the base URL above."
              >
                <input
                  type="password"
                  value={llm.apiKey}
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
                  value={llm.model}
                  onChange={(e) => {
                    patch({ model: e.target.value });
                  }}
                />
                {llm.provider === 'ollama' && (
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
                      className={`chip ${llm.model === name ? 'chip-active' : ''}`}
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
                value={llm.timeoutSecs}
                onChange={(e) => {
                  patch({ timeoutSecs: Math.max(5, Number(e.target.value) || 30) });
                }}
              />
            </Row>
            <div className="row-actions">
              <button
                className="btn"
                disabled={testing || urlInvalid}
                onClick={() => void runTest()}
              >
                {testing ? 'Testing…' : 'Test connection'}
              </button>
              {testResult && (
                <span className={testResult.ok ? 'badge badge-ok' : 'form-error'}>
                  {testResult.message}
                </span>
              )}
            </div>
            {llm.provider === 'openaiCompatible' && (
              <p className="privacy-note">
                Heads up: with a cloud provider, transcribed text (never audio) is sent to the
                endpoint above for refinement.
              </p>
            )}
          </>
        )}
      </section>
    </div>
  );
}
