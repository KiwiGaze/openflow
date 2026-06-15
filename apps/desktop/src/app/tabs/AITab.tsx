import { useEffect, useState, type JSX } from 'react';
import { isLocalEndpoint, LLM_PROFILE_VERSION, type LlmProfile, OLLAMA_PRESET } from '@velata/core';
import type { SettingsApi } from '../hooks.js';
import { useLlmProfiles } from '../hooks.js';
import { ipc } from '../ipc.js';
import { Callout } from '../components/Callout.js';
import { LlmProfileEditor } from '../components/LlmProfileEditor.js';

export function AITab({ api }: { api: SettingsApi }): JSX.Element {
  const { settings, update } = api;
  const { profiles, save, remove } = useLlmProfiles();
  const [selectedId, setSelectedId] = useState<string | null>(null);

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
