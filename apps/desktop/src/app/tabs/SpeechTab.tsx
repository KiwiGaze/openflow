import { type JSX } from 'react';
import { formatBytes, formatProgress } from '@velata/core';
import type { ModelsApi, SettingsApi } from '../hooks.js';
import { downloadFailed, DOWNLOAD_FAILED_HINT, isDownloading } from '../modelStatus.js';
import { Callout } from '../components/Callout.js';
import { SttEngines } from '../components/SttEngines.js';

export function SpeechTab({
  api,
  modelsApi,
}: {
  api: SettingsApi;
  modelsApi: ModelsApi;
}): JSX.Element {
  const { settings, update } = api;
  const { models, progress, download, cancel, remove: removeModel } = modelsApi;

  // models is empty until the first list arrives; only warn once we know.
  const noModelInstalled = models.length > 0 && !models.some((m) => m.installed);

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
    </div>
  );
}
