import { useEffect, useState, type JSX } from 'react';
import type { AppInfo } from '@velata/core';
import { ipc } from '../ipc.js';
import { WHATS_NEW } from '../whatsNew.js';

export function AboutTab(): JSX.Element {
  const [info, setInfo] = useState<AppInfo | null>(null);

  useEffect(() => {
    void ipc.getAppInfo().then(setInfo);
  }, []);

  return (
    <div className="tab-body">
      <section className="card">
        <h2>About Velata</h2>
        <p className="row-hint">
          Local-first AI voice input. Hold a key, speak, and get clean text in any app.
        </p>
        {info && (
          <dl className="about-list">
            <dt>Version</dt>
            <dd>{info.version}</dd>
            <dt>Settings file</dt>
            <dd className="mono">{info.configPath}</dd>
            <dt>Data folder (models)</dt>
            <dd className="mono">{info.dataDir}</dd>
            <dt>Source code</dt>
            <dd className="mono">github.com/KiwiGaze/velata</dd>
          </dl>
        )}
      </section>

      <section className="card">
        <h2>What's new</h2>
        {WHATS_NEW.map((note, index) => (
          <div key={index} className="whats-new-entry">
            <div className="whats-new-meta">
              Version {note.version} · {note.date}
            </div>
            <ul className="privacy-list">
              {note.highlights.map((highlight) => (
                <li key={highlight}>{highlight}</li>
              ))}
            </ul>
          </div>
        ))}
      </section>

      <section className="card">
        <h2>Privacy</h2>
        <ul className="privacy-list">
          <li>Dictation runs on this Mac. Audio never leaves your device and is never saved.</li>
          <li>AI Transforms send only text to your AI provider.</li>
          <li>
            Cloud speech (only if you enable it) uploads audio to that provider — and tells you
            first.
          </li>
        </ul>
      </section>
    </div>
  );
}
