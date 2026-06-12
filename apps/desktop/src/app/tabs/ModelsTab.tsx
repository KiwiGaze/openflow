import { useEffect, useState, type JSX } from 'react';
import {
  formatBytes,
  formatProgress,
  isLocalEndpoint,
  LLM_PROFILE_VERSION,
  type LlmProfile,
  OLLAMA_PRESET,
} from '@velata/core';
import type { ModelsApi, SettingsApi } from '../hooks.js';
import { useLlmProfiles } from '../hooks.js';
import { ipc } from '../ipc.js';
import { downloadFailed, DOWNLOAD_FAILED_HINT, isDownloading } from '../modelStatus.js';
import { Callout } from '../components/Callout.js';
import { LlmProfileEditor } from '../components/LlmProfileEditor.js';
import { SttEngines } from '../components/SttEngines.js';

export function ModelsTab({
  api,
  modelsApi,
}: {
  api: SettingsApi;
  modelsApi: ModelsApi;
}): JSX.Element {
  const { settings, update } = api;
  const { models, progress, download, cancel, remove: removeModel } = modelsApi;
  const { profiles, save, remove } = useLlmProfiles();
  const [selectedId, setSelectedId] = useState<string | null>(null);

  // models is empty until the first list arrives; only warn once we know.
  const noModelInstalled = models.length > 0 && !models.some((m) => m.installed);

  const selected = profiles.find((p) => p.id === selectedId) ?? null;

  const addProfile = (opts?: { name?: string; model?: string | undefined }): void => {
    const profile: LlmProfile = {
      version: LLM_PROFILE_VERSION,
      id: crypto.randomUUID(),
      name: opts?.name ?? 'New profile',
      provider: OLLAMA_PRESET.kind,
      baseUrl: OLLAMA_PRESET.baseUrl,
      apiKey: '',
      model: opts?.model ?? OLLAMA_PRESET.modelSuggestion,
      timeoutSecs: 30,
      presetId: OLLAMA_PRESET.id,
    };
    setSelectedId(profile.id);
    void save(profile).then(() => {
      // The first profile is what the user came for: select it for polish.
      if (settings.activeLlmProfileId === '') void update({ activeLlmProfileId: profile.id });
    });
  };

  const deleteProfile = (id: string): void => {
    if (selectedId === id) setSelectedId(null);
    // Deleting the active profile clears the selection backend-side.
    void remove(id);
  };

  // One probe of the default Ollama endpoint on mount — a quick-add nudge, not
  // a storm. Removes the last manual step from the biggest single upgrade.
  const [ollamaDetected, setOllamaDetected] = useState<string[] | null>(null);
  useEffect(() => {
    void ipc.listOllamaModels(OLLAMA_PRESET.baseUrl).then(
      (found) => {
        setOllamaDetected(found);
      },
      () => {
        setOllamaDetected(null);
      },
    );
  }, []);

  return (
    <div className="tab-body">
      <section className="card">
        <h2>Speech recognition</h2>
        {noModelInstalled && (
          <Callout variant="warn">
            No speech model installed — dictation is disabled. Download one below.
          </Callout>
        )}
        <div className="model-list">
          {models.map((model) => {
            const p = progress[model.id];
            const downloading = isDownloading(model, p);
            const failed = downloadFailed(p);
            const active = settings.sttModelId === model.id;
            return (
              <div key={model.id} className={`model-row ${active ? 'model-active' : ''}`}>
                <label className="model-pick">
                  <input
                    type="radio"
                    name="stt-model"
                    checked={active}
                    disabled={!model.installed}
                    onChange={() => void update({ sttModelId: model.id })}
                  />
                  <div>
                    <div className="row-title">
                      {model.displayName}
                      {model.multilingual && <span className="badge">multilingual</span>}
                    </div>
                    <div className="row-hint">
                      {formatBytes(model.sizeBytes)} — {model.description}
                    </div>
                    {failed && <div className="row-hint row-hint-warn">{DOWNLOAD_FAILED_HINT}</div>}
                  </div>
                </label>
                <div className="model-actions">
                  {model.installed && !active && (
                    <button className="btn btn-quiet" onClick={() => void removeModel(model.id)}>
                      Delete
                    </button>
                  )}
                  {model.installed && <span className="badge badge-ok">installed</span>}
                  {!model.installed && downloading && (
                    <>
                      <span className="row-hint">
                        {p ? formatProgress(p.downloadedBytes, p.totalBytes) : '…'}
                      </span>
                      <button
                        className="btn btn-quiet"
                        onClick={() => {
                          cancel(model.id);
                        }}
                      >
                        Cancel
                      </button>
                    </>
                  )}
                  {!model.installed && !downloading && (
                    <button
                      className="btn"
                      onClick={() => {
                        download(model.id);
                      }}
                    >
                      {failed ? 'Retry' : 'Download'}
                    </button>
                  )}
                </div>
              </div>
            );
          })}
        </div>
      </section>

      <SttEngines api={api} />

      <section className="card">
        <h2>AI profiles</h2>
        {profiles.length === 0 && (
          <p className="row-hint">
            Dictation uses fast rules-based cleanup. Add a profile to enable AI polish and the
            selection shortcuts — nothing leaves your Mac unless you pick a cloud endpoint.
          </p>
        )}
        {profiles.length === 0 && ollamaDetected && ollamaDetected.length > 0 && (
          <Callout
            variant="info"
            action={{
              label: 'Add Ollama',
              onClick: () => {
                addProfile({ name: 'Ollama (local)', model: ollamaDetected[0] });
              },
            }}
          >
            Ollama is running locally with {ollamaDetected.length} model
            {ollamaDetected.length === 1 ? '' : 's'}. Add it for on-device AI polish — nothing
            leaves your Mac.
          </Callout>
        )}
        <div className="mode-list">
          <div className="mode-row">
            <label>
              <input
                type="radio"
                name="active-profile"
                aria-label="No AI — rules-based cleanup only"
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
                setSelectedId(profile.id);
              }}
              onKeyDown={(e) => {
                // Keys on the nested radio bubble up here; leave those alone.
                if (e.target !== e.currentTarget) return;
                if (e.key === 'Enter' || e.key === ' ') {
                  e.preventDefault();
                  setSelectedId(profile.id);
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
                  aria-label={`Use ${profile.name}`}
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
          <button
            className="btn"
            onClick={() => {
              addProfile();
            }}
          >
            New profile
          </button>
        </div>
      </section>

      {selected && (
        <LlmProfileEditor
          key={selected.id}
          profile={selected}
          onSave={save}
          onDelete={() => {
            deleteProfile(selected.id);
          }}
        />
      )}
    </div>
  );
}
