import { useMemo, useState, type JSX } from 'react';
import { formatAcceleratorMac, formatBytes, formatProgress } from '@openflow/core';
import type { ModelsApi, SettingsApi } from './hooks.js';
import { usePermissions, usePipeline } from './hooks.js';
import { ipc } from './ipc.js';

const STEPS = ['Welcome', 'Microphone', 'Accessibility', 'Speech model', 'Try it'] as const;

/** Models offered during onboarding; everything else lives in Settings. */
const STARTER_MODELS = ['base.en', 'small.en', 'large-v3-turbo-q5_0'];

export function Onboarding({
  api,
  modelsApi,
}: {
  api: SettingsApi;
  modelsApi: ModelsApi;
}): JSX.Element {
  const { settings, update } = api;
  const [step, setStep] = useState(0);
  const permissions = usePermissions();
  const { state, lastResult } = usePipeline();
  const { models, progress, download } = modelsApi;

  const starterModels = useMemo(
    () => models.filter((m) => STARTER_MODELS.includes(m.id)),
    [models],
  );
  const selectedModel = models.find((m) => m.id === settings.sttModelId);
  const finish = (): void => {
    void update({ onboardingCompleted: true });
  };

  const micGranted = permissions?.microphone === 'granted';
  const micDenied = permissions?.microphone === 'denied';

  return (
    <div className="onboarding">
      <div className="onboarding-steps">
        {STEPS.map((name, i) => (
          <span
            key={name}
            className={`step-dot ${i === step ? 'step-current' : ''} ${i < step ? 'step-done' : ''}`}
          >
            {name}
          </span>
        ))}
      </div>

      {step === 0 && (
        <div className="onboarding-pane">
          <h1>Welcome to OpenFlow</h1>
          <p>
            Hold <strong>{formatAcceleratorMac(settings.dictationHotkey)}</strong>, speak naturally,
            release — and clean text lands in whatever app you're using.
          </p>
          <ul className="privacy-list">
            <li>Your voice is transcribed entirely on this Mac.</li>
            <li>No account, no telemetry, no cloud by default.</li>
            <li>Optional AI polish via Ollama (local) or your own API key.</li>
          </ul>
        </div>
      )}

      {step === 1 && (
        <div className="onboarding-pane">
          <h1>Microphone access</h1>
          <p>OpenFlow records only while you hold the hotkey. Audio is never written to disk.</p>
          <div className="perm-status">
            Status:{' '}
            <span className={micGranted ? 'badge badge-ok' : 'badge'}>
              {permissions?.microphone ?? 'checking…'}
            </span>
          </div>
          {!micGranted && !micDenied && (
            <button
              className="btn btn-primary"
              onClick={() => void ipc.requestMicrophonePermission()}
            >
              Allow microphone
            </button>
          )}
          {micDenied && (
            <button className="btn" onClick={() => void ipc.openMicrophoneSettings()}>
              Open System Settings
            </button>
          )}
          <p className="row-hint">
            Running from `tauri dev`? The permission attaches to your terminal app.
          </p>
        </div>
      )}

      {step === 2 && (
        <div className="onboarding-pane">
          <h1>Accessibility access</h1>
          <p>
            OpenFlow pastes text by simulating ⌘V, which macOS gates behind the Accessibility
            permission. It also lets OpenFlow read your selected text for the Rewrite and Polish
            hotkeys. Skip this and results are copied to the clipboard, and Rewrite and Polish won't
            work.
          </p>
          <div className="perm-status">
            Status:{' '}
            <span className={permissions?.accessibility ? 'badge badge-ok' : 'badge'}>
              {permissions ? (permissions.accessibility ? 'granted' : 'not granted') : 'checking…'}
            </span>
          </div>
          {!permissions?.accessibility && (
            <div className="row-actions">
              <button
                className="btn btn-primary"
                onClick={() => void ipc.promptAccessibilityPermission()}
              >
                Grant access
              </button>
              <button className="btn" onClick={() => void ipc.openAccessibilitySettings()}>
                Open System Settings
              </button>
            </div>
          )}
        </div>
      )}

      {step === 3 && (
        <div className="onboarding-pane">
          <h1>Pick a speech model</h1>
          <p>
            Downloaded once from Hugging Face, then everything runs offline. You can switch anytime.
          </p>
          <div className="model-list">
            {starterModels.map((model) => {
              const p = progress[model.id];
              const downloading = (model.downloading || (p && !p.done)) ?? false;
              const failed = (p?.done && p.error) ?? false;
              return (
                <label key={model.id} className="model-row">
                  <input
                    type="radio"
                    name="onboarding-model"
                    checked={settings.sttModelId === model.id}
                    onChange={() => void update({ sttModelId: model.id })}
                  />
                  <div className="model-pick">
                    <div>
                      <div className="row-title">{model.displayName}</div>
                      <div className="row-hint">
                        {formatBytes(model.sizeBytes)} — {model.description}
                      </div>
                      {failed && (
                        <div className="row-hint row-hint-warn">
                          Download failed — check your connection.
                        </div>
                      )}
                    </div>
                  </div>
                  <div className="model-actions">
                    {model.installed && <span className="badge badge-ok">installed</span>}
                    {!model.installed && downloading && (
                      <span className="row-hint">
                        {p ? formatProgress(p.downloadedBytes, p.totalBytes) : '…'}
                      </span>
                    )}
                    {!model.installed && !downloading && (
                      <button
                        className="btn"
                        onClick={(e) => {
                          e.preventDefault();
                          download(model.id);
                        }}
                      >
                        {failed ? 'Retry' : 'Download'}
                      </button>
                    )}
                  </div>
                </label>
              );
            })}
          </div>
          {!selectedModel?.installed && (
            <p className="row-hint">Download the selected model to continue.</p>
          )}
        </div>
      )}

      {step === 4 && (
        <div className="onboarding-pane">
          <h1>Try it</h1>
          <p>
            Click into any text field (Notes, a browser, anywhere), hold{' '}
            <strong>{formatAcceleratorMac(settings.dictationHotkey)}</strong>, say something, and
            release.
          </p>
          <div className="perm-status">
            Pipeline: <span className="badge">{state.status}</span>
          </div>
          {lastResult && (
            <div className="card">
              <div className="row-title">It heard:</div>
              <p className="result-text">{lastResult.text}</p>
            </div>
          )}
        </div>
      )}

      <div className="onboarding-nav">
        <button className="btn btn-quiet" onClick={finish}>
          Skip setup
        </button>
        <div className="row-actions">
          {step > 0 && (
            <button
              className="btn"
              onClick={() => {
                setStep(step - 1);
              }}
            >
              Back
            </button>
          )}
          {step < STEPS.length - 1 ? (
            <button
              className="btn btn-primary"
              disabled={step === 3 && !selectedModel?.installed}
              onClick={() => {
                setStep(step + 1);
              }}
            >
              Continue
            </button>
          ) : (
            <button className="btn btn-primary" onClick={finish}>
              Finish
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
