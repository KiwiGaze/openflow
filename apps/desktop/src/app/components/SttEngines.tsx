import { useState, type JSX } from 'react';
import {
  CLOUD_STT_PREFIX,
  DEFAULT_STT_PRESET,
  STT_PRESETS,
  STT_PROFILE_VERSION,
  clampTimeoutSecs,
  isLocalEndpoint,
  isValidBaseUrl,
  type SttProfile,
} from '@openflow/core';
import type { SettingsApi } from '../hooks.js';
import { useSttProfiles } from '../hooks.js';
import { ipc } from '../ipc.js';
import { Callout } from './Callout.js';
import { Row } from './Row.js';

function hostOf(url: string): string {
  try {
    return new URL(url).host;
  } catch {
    return 'the provider';
  }
}

/**
 * Cloud / remote STT engines (08). The on-device whisper models live in the
 * Speech recognition card; this is the opt-in, audio-leaves-the-Mac surface,
 * with the consent gate that the pipeline also enforces (08 §3).
 */
export function SttEngines({ api }: { api: SettingsApi }): JSX.Element {
  const { settings, update } = api;
  const { profiles, save, remove } = useSttProfiles();
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [consentFor, setConsentFor] = useState<SttProfile | null>(null);

  const selected = profiles.find((p) => p.id === selectedId) ?? null;
  const activeId = settings.sttModelId.startsWith(CLOUD_STT_PREFIX)
    ? settings.sttModelId.slice(CLOUD_STT_PREFIX.length)
    : null;

  const addProfile = (): void => {
    const profile: SttProfile = {
      version: STT_PROFILE_VERSION,
      id: crypto.randomUUID(),
      name: 'New engine',
      engine: 'openaiAudio',
      presetId: DEFAULT_STT_PRESET.id,
      baseUrl: DEFAULT_STT_PRESET.baseUrl,
      apiKey: '',
      model: DEFAULT_STT_PRESET.model,
      timeoutSecs: 30,
    };
    setSelectedId(profile.id);
    void save(profile);
  };

  const deleteProfile = async (id: string): Promise<void> => {
    await remove(id);
    if (selectedId === id) setSelectedId(null);
  };

  // Selecting a cloud engine for the first time gates on consent; only after
  // confirming does sttModelId become `cloud:<id>` (and the id join the
  // confirmed set the pipeline checks).
  const selectEngine = (profile: SttProfile): void => {
    const cloud = !isLocalEndpoint(profile.baseUrl);
    if (cloud && !settings.confirmedSttProfiles.includes(profile.id)) {
      setConsentFor(profile);
      return;
    }
    void update({ sttModelId: `${CLOUD_STT_PREFIX}${profile.id}` });
  };

  const confirmConsent = (profile: SttProfile): void => {
    void update({
      sttModelId: `${CLOUD_STT_PREFIX}${profile.id}`,
      confirmedSttProfiles: [...settings.confirmedSttProfiles, profile.id],
    });
    setConsentFor(null);
  };

  const patch = (next: Partial<SttProfile>): void => {
    if (selected) void save({ ...selected, ...next });
  };

  const applyPreset = (presetId: string): void => {
    if (!selected) return;
    const preset = STT_PRESETS.find((p) => p.id === presetId);
    if (!preset) return;
    const next: SttProfile = { ...selected, presetId };
    if (preset.id !== 'custom') {
      next.baseUrl = preset.baseUrl;
      next.model = preset.model;
    }
    void save(next);
  };

  return (
    <section className="card">
      <h2>Cloud &amp; remote speech engines</h2>
      <p className="row-hint">
        By default OpenFlow transcribes on this Mac. Add a cloud or local-server engine to
        transcribe elsewhere — cloud engines upload your audio, and OpenFlow asks first.
      </p>
      {consentFor && (
        <Callout variant="warn">
          <strong>Send your audio to a cloud service?</strong> OpenFlow normally transcribes on your
          Mac. With <strong>{consentFor.name}</strong>, each time you dictate the full recording is
          uploaded to {hostOf(consentFor.baseUrl)} using your key. OpenFlow stores nothing; that
          service&rsquo;s policy governs the audio. The on-device engine stays the default.{' '}
          <button
            className="btn btn-sm"
            onClick={() => {
              confirmConsent(consentFor);
            }}
          >
            Use {consentFor.name} (uploads audio)
          </button>{' '}
          <button
            className="btn btn-quiet btn-sm"
            onClick={() => {
              setConsentFor(null);
            }}
          >
            Keep on-device
          </button>
        </Callout>
      )}
      {profiles.length > 0 && (
        <div className="mode-list" role="radiogroup" aria-label="Cloud speech engine">
          {profiles.map((p) => {
            const cloud = !isLocalEndpoint(p.baseUrl);
            return (
              <div key={p.id} className={`mode-row ${selectedId === p.id ? 'mode-selected' : ''}`}>
                <input
                  type="radio"
                  name="stt-engine"
                  aria-label={`Use ${p.name}`}
                  checked={activeId === p.id}
                  onChange={() => {
                    selectEngine(p);
                  }}
                />
                <button
                  type="button"
                  className="mode-edit"
                  onClick={() => {
                    setSelectedId(p.id);
                  }}
                >
                  {p.name}
                </button>
                {cloud ? (
                  <span className="badge badge-audio">cloud — audio leaves this Mac</span>
                ) : (
                  <span className="badge">local</span>
                )}
                {p.model && <span className="badge badge-muted">{p.model}</span>}
              </div>
            );
          })}
        </div>
      )}
      <div className="row-actions">
        <button className="btn btn-quiet" onClick={() => void ipc.revealSttProfiles()}>
          Show in Finder
        </button>
        <button className="btn" onClick={addProfile}>
          Add engine
        </button>
      </div>

      {selected && (
        <div className="prompt-edit">
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
              value={selected.presetId || 'custom'}
              onChange={(e) => {
                applyPreset(e.target.value);
              }}
            >
              {STT_PRESETS.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.label}
                </option>
              ))}
            </select>
          </Row>
          <Row
            title="Base URL"
            hint="OpenAI-audio endpoint root, e.g. https://api.groq.com/openai/v1."
          >
            <input
              type="text"
              className={isValidBaseUrl(selected.baseUrl) ? '' : 'input-invalid'}
              value={selected.baseUrl}
              onChange={(e) => {
                patch({ baseUrl: e.target.value });
              }}
            />
          </Row>
          <Row title="API key" hint="Stored in the profile file. Sent only to the base URL above.">
            <input
              type="password"
              value={selected.apiKey}
              autoComplete="off"
              onChange={(e) => {
                patch({ apiKey: e.target.value });
              }}
            />
          </Row>
          <Row title="Model">
            <input
              type="text"
              value={selected.model}
              onChange={(e) => {
                patch({ model: e.target.value });
              }}
            />
          </Row>
          <Row title="Timeout" hint="Seconds to wait for a response.">
            <input
              type="number"
              min={5}
              max={300}
              value={selected.timeoutSecs}
              onChange={(e) => {
                patch({ timeoutSecs: clampTimeoutSecs(e.target.value) });
              }}
            />
          </Row>
          <div className="row-actions">
            <button
              className="btn btn-danger"
              onClick={() => {
                void deleteProfile(selected.id);
              }}
            >
              Delete
            </button>
          </div>
        </div>
      )}
    </section>
  );
}
