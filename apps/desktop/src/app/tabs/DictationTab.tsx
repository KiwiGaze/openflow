import type { JSX } from 'react';
import { type HotkeyBehavior, LANGUAGES } from '@velata/core';
import type { ModelsApi, SettingsApi } from '../hooks.js';
import { HotkeyRecorder } from '../components/HotkeyRecorder.js';
import { Row } from '../components/Row.js';
import { Toggle } from '../components/Toggle.js';

export function DictationTab({
  api,
  modelsApi,
}: {
  api: SettingsApi;
  modelsApi: ModelsApi;
}): JSX.Element {
  const { settings, update } = api;
  const { models } = modelsApi;
  const activeModel = models.find((m) => m.id === settings.sttModelId);
  const englishOnly = (activeModel && !activeModel.multilingual) ?? false;

  return (
    <div className="tab-body">
      <section className="card">
        <h2>Hotkeys</h2>
        <Row
          title="Dictation"
          hint="Hold to talk; release to insert. Tip: a quick tap keeps recording hands-free until you tap again."
        >
          <HotkeyRecorder
            value={settings.dictationHotkey}
            label="Dictation"
            onChange={(accelerator) => void update({ dictationHotkey: accelerator })}
          />
        </Row>
        <Row
          title="When I press the hotkey"
          hint="Hold to talk, or tap once to start and again to stop."
        >
          <select
            value={settings.dictationHotkeyBehavior}
            onChange={(e) =>
              void update({ dictationHotkeyBehavior: e.target.value as HotkeyBehavior })
            }
          >
            <option value="hold">Hold to talk</option>
            <option value="toggle">Tap to start, tap to stop</option>
          </select>
        </Row>
        <Row
          title="See changes"
          hint="Reveal a word-level diff of the last cleanup or polish. Empty disables it."
        >
          <HotkeyRecorder
            value={settings.changeOverlayHotkey}
            label="See changes"
            onChange={(accelerator) => void update({ changeOverlayHotkey: accelerator })}
          />
        </Row>
        <p className="row-hint">Press Esc while recording to cancel.</p>
      </section>

      <section className="card">
        <h2>Speech</h2>
        <Row
          title="Spoken language"
          hint={
            englishOnly
              ? `${activeModel?.displayName ?? 'This model'} only transcribes English — switch to a multilingual model to dictate in another language.`
              : 'The language you’ll speak, or Auto-detect. A mode can override this.'
          }
        >
          <select
            value={settings.language}
            disabled={englishOnly}
            onChange={(e) => void update({ language: e.target.value })}
          >
            {LANGUAGES.map(([code, name]) => (
              <option key={code} value={code}>
                {name}
              </option>
            ))}
          </select>
        </Row>
      </section>

      <section className="card">
        <h2>After transcribing</h2>
        <Row
          title="Polish with AI"
          hint="Polish transcripts with your active mode and AI profile. Off = fast rules-based cleanup, no network."
        >
          <Toggle
            checked={settings.polishAfterDictation}
            onChange={(checked) => void update({ polishAfterDictation: checked })}
            label="Polish with AI"
          />
        </Row>
      </section>
    </div>
  );
}
