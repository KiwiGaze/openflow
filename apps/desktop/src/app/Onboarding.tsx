import { useState, type JSX } from 'react';
import {
  effectiveAccelerator,
  formatAcceleratorMac,
  formatBytes,
  formatProgress,
  PUSH_TO_TALK_FALLBACK,
} from '@velata/core';
import type { ModelsApi, SettingsApi } from './hooks.js';
import { usePermissions, usePipeline } from './hooks.js';
import { ipc } from './ipc.js';
import { downloadFailed, DOWNLOAD_FAILED_HINT, isDownloading } from './modelStatus.js';

const STEPS = ['Welcome', 'Microphone', 'Try dictation', 'Done'] as const;

/** Models offered on first run; everything else lives in Settings → Speech. */
const STARTER_MODELS = ['base.en', 'small.en', 'large-v3-turbo-q5_0'];
const DEFAULT_MODEL = 'base.en';

export function Onboarding({
  api,
  modelsApi,
}: {
  api: SettingsApi;
  modelsApi: ModelsApi;
}): JSX.Element {
  const { settings, update } = api;
  const [step, setStep] = useState(0);
  const [skipping, setSkipping] = useState(false);
  const [showOtherModels, setShowOtherModels] = useState(false);
  const permissions = usePermissions();
  const { lastResult } = usePipeline();
  const { models, progress, download } = modelsApi;

  const starterModels = models.filter((m) => STARTER_MODELS.includes(m.id));
  const baseModel = models.find((m) => m.id === DEFAULT_MODEL);
  // Show the shortcut push-to-talk effectively fires under today: the fn gesture
  // default can't be observed yet, so onboarding teaches the fallback accelerator
  // that actually works (Phase 3 adds real fn observation).
  const hotkey = formatAcceleratorMac(
    effectiveAccelerator(settings.pushToTalkHotkey, PUSH_TO_TALK_FALLBACK) ?? PUSH_TO_TALK_FALLBACK,
  );

  const micGranted = permissions?.microphone === 'granted';
  const micDenied = permissions?.microphone === 'denied';
  const axGranted = permissions?.accessibility === true;
  const activeProgress = progress[settings.sttModelId];
  const modelInstalled = models.find((m) => m.id === settings.sttModelId)?.installed ?? false;
  const modelReadyPct = activeProgress
    ? formatProgress(activeProgress.downloadedBytes, activeProgress.totalBytes)
    : '…';

  const finish = (): void => {
    void update({ onboardingCompleted: true });
  };

  if (skipping) {
    const modelLine = modelInstalled
      ? '✓ Speech model: ready'
      : activeProgress && !activeProgress.done
        ? '• Speech model: still downloading — it will finish in the background'
        : '✗ Speech model: not downloaded — dictation won’t work until you download one in Settings → Speech.';
    return (
      <div className="onboarding">
        <div className="onboarding-pane">
          <h1>Skipping setup</h1>
          <p>Here’s where things stand:</p>
          <ul className="privacy-list">
            <li>{micGranted ? '✓ Microphone: granted' : '✗ Microphone: not granted yet'}</li>
            <li>{modelLine}</li>
            <li>
              {axGranted
                ? '✓ Accessibility: on — Velata pastes for you'
                : '• Accessibility: off — text will go to your clipboard (⌘V)'}
            </li>
          </ul>
        </div>
        <div className="onboarding-nav">
          <button
            className="btn"
            onClick={() => {
              setSkipping(false);
            }}
          >
            Back to setup
          </button>
          <button className="btn btn-primary" onClick={finish}>
            Skip anyway
          </button>
        </div>
      </div>
    );
  }

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
          <h1>Welcome to Velata</h1>
          <p>
            Hold <strong>{hotkey}</strong>, speak, release — clean text lands in whatever app you’re
            using.
          </p>
          <ul className="privacy-list">
            <li>Your voice is transcribed on this Mac. Audio never leaves.</li>
            <li>No account, no telemetry, no cloud by default.</li>
            <li>Optional AI polish via Ollama (local) or your own API key.</li>
          </ul>
          <p>
            To transcribe, Velata needs a speech model (148 MB, one time). It downloads from Hugging
            Face, then runs on this Mac. You can manage models later in Settings → Speech.
          </p>
          <div className="row-actions">
            <DownloadConsent baseModel={baseModel} progress={progress} download={download} />
            <button
              className="btn btn-quiet"
              onClick={() => {
                setShowOtherModels((v) => !v);
              }}
            >
              {showOtherModels ? 'Hide other models' : 'Choose another model'}
            </button>
          </div>
          {showOtherModels && (
            <div className="model-list">
              {starterModels.map((model) => {
                const p = progress[model.id];
                const downloading = isDownloading(model, p);
                const failed = downloadFailed(p);
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
                          <div className="row-hint row-hint-warn">{DOWNLOAD_FAILED_HINT}</div>
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
          )}
        </div>
      )}

      {step === 1 && (
        <div className="onboarding-pane">
          <h1>Let Velata hear you</h1>
          <p>Velata records only while you hold the hotkey. Audio is never written to disk.</p>
          <div className="perm-status">
            Microphone:{' '}
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
            <>
              <p className="row-hint">
                macOS is blocking the microphone. Open System Settings → Privacy &amp; Security →
                Microphone, switch Velata on, then come back — this updates automatically.
              </p>
              <button className="btn" onClick={() => void ipc.openMicrophoneSettings()}>
                Open System Settings
              </button>
            </>
          )}
        </div>
      )}

      {step === 2 && (
        <div className="onboarding-pane">
          <h1>Try your first dictation</h1>
          {micDenied ? (
            <>
              <p className="row-hint row-hint-warn">
                Microphone is off, so there’s nothing to transcribe. Turn it on in System Settings →
                Microphone, then try again.
              </p>
              <button className="btn" onClick={() => void ipc.openMicrophoneSettings()}>
                Open System Settings
              </button>
            </>
          ) : !modelInstalled ? (
            <p className="row-hint">Getting the speech model ready… {modelReadyPct}</p>
          ) : (
            <>
              <p>
                Click the box below, then hold <strong>{hotkey}</strong> and say:{' '}
                <em>“this is my first note with Velata”</em>
              </p>
              <textarea
                className="tryit-field"
                rows={3}
                placeholder="(your dictation appears here)"
                defaultValue=""
              />
              {lastResult && <p className="result-text">{lastResult.text}</p>}
              {lastResult && !axGranted && (
                <p className="row-hint">
                  Copied to your clipboard. Click the box and press ⌘V to drop it in.
                </p>
              )}
            </>
          )}
        </div>
      )}

      {step === 3 && (
        <div className="onboarding-pane">
          <div className="youre-set-head">
            <h1>That’s dictation.</h1>
            <span className="row-hint">↑ Velata lives in your menu bar</span>
          </div>
          {lastResult ? (
            <div className="diff-panel">
              <div>
                <p className="result-text">{lastResult.raw}</p>
                <div className="row-hint">raw transcript</div>
              </div>
              <div className="dict-arrow">→</div>
              <div>
                <p className="result-text">{lastResult.text}</p>
                <div className="row-hint">
                  {lastResult.polished ? 'polished with AI' : 'cleaned up'}
                </div>
              </div>
            </div>
          ) : (
            <p>
              Hold <strong>{hotkey}</strong> in any app to dictate.
            </p>
          )}
          <p className="row-title">A couple of things worth knowing:</p>
          <ul className="privacy-list">
            <li>
              Tap <strong>{hotkey}</strong> (don’t hold) to keep recording hands-free.
            </li>
            <li>
              {axGranted
                ? 'Velata pastes straight into the active app for you.'
                : 'Grant Accessibility in System Settings later and Velata will paste for you — until then your text waits on the clipboard for ⌘V.'}
            </li>
          </ul>
        </div>
      )}

      <div className="onboarding-nav">
        <button
          className="btn btn-quiet"
          onClick={() => {
            setSkipping(true);
          }}
        >
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
              onClick={() => {
                setStep(step + 1);
              }}
            >
              Continue
            </button>
          ) : (
            <button className="btn btn-primary" onClick={finish}>
              Start using Velata
            </button>
          )}
        </div>
      </div>
    </div>
  );
}

/** The Welcome download button: idle → progress → ready, with Retry on failure. */
function DownloadConsent({
  baseModel,
  progress,
  download,
}: {
  baseModel: ModelsApi['models'][number] | undefined;
  progress: ModelsApi['progress'];
  download: ModelsApi['download'];
}): JSX.Element {
  const p = progress[DEFAULT_MODEL];
  const installed = baseModel?.installed ?? false;
  const downloading = isDownloading(baseModel, p);
  const failed = downloadFailed(p);

  if (installed) {
    return <span className="badge badge-ok">Base (English) ready ✓</span>;
  }
  if (downloading) {
    return (
      <span className="row-hint">
        Downloading Base (English)… {p ? formatProgress(p.downloadedBytes, p.totalBytes) : '…'}
      </span>
    );
  }
  return (
    <button
      className="btn btn-primary"
      onClick={() => {
        download(DEFAULT_MODEL);
      }}
    >
      {failed ? 'Download failed — Retry' : 'Download Base (English)'}
    </button>
  );
}
