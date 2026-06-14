import { useEffect, useState, type JSX } from 'react';
import {
  formatAcceleratorMac,
  formatHotkey,
  HANDS_FREE_FALLBACK,
  LANGUAGES,
  PUSH_TO_TALK_FALLBACK,
  type Hotkey,
} from '@velata/core';
import type { ModelsApi, SettingsApi } from '../hooks.js';
import { ipc } from '../ipc.js';
import { HotkeyRecorder } from '../components/HotkeyRecorder.js';
import { Row } from '../components/Row.js';

/**
 * Edits a dictation gesture trigger (push-to-talk / hands-free). The `fn`-key
 * gesture defaults can't be observed yet (Phase 3), so a gesture trigger is
 * shown read-only with its fallback note, plus a way to record an accelerator
 * instead; an accelerator trigger is editable and clearable to disable it.
 */
function GestureHotkey({
  label,
  hotkey,
  fallbackHint,
  onChange,
}: {
  label: string;
  hotkey: Hotkey;
  /** What works today while the gesture can't be observed, e.g. "⌥ Space". */
  fallbackHint: string;
  onChange: (next: Hotkey) => void;
}): JSX.Element {
  const isGesture = hotkey.kind !== 'accelerator';
  if (isGesture) {
    return (
      <div className="hotkey-gesture">
        <span className="hotkey-gesture-current">{formatHotkey(hotkey)}</span>
        <p className="row-hint">
          The fn key needs Input Monitoring (coming soon). For now, {fallbackHint} works.
        </p>
        <button
          type="button"
          className="btn btn-quiet"
          onClick={() => {
            onChange({ kind: 'accelerator', key: '' });
          }}
        >
          Set a shortcut instead
        </button>
      </div>
    );
  }
  return (
    <div className="hotkey-gesture">
      <HotkeyRecorder
        value={hotkey.key}
        label={label}
        emptyLabel="Set…"
        onChange={(accelerator) => {
          onChange({ kind: 'accelerator', key: accelerator });
        }}
      />
      {hotkey.key.trim() !== '' && (
        <button
          type="button"
          className="btn btn-quiet"
          aria-label={`Clear ${label} shortcut`}
          onClick={() => {
            onChange({ kind: 'accelerator', key: '' });
          }}
        >
          Clear
        </button>
      )}
    </div>
  );
}

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

  // Enumerate input devices once when the tab mounts (local hardware; no
  // polling). An empty list disables the picker with "System default" only.
  const [inputDevices, setInputDevices] = useState<string[]>([]);
  useEffect(() => {
    void ipc
      .listInputDevices()
      .then(setInputDevices)
      .catch((err: unknown) => {
        console.error('Failed to list input devices:', err);
      });
  }, []);

  return (
    <div className="tab-body">
      <section className="card">
        <h2>Hotkeys</h2>
        <Row
          title="Push to talk"
          hint="Hold to talk; release to insert. Tip: a quick tap keeps recording hands-free until you tap again."
        >
          <GestureHotkey
            label="Push to talk"
            hotkey={settings.pushToTalkHotkey}
            fallbackHint={formatAcceleratorMac(PUSH_TO_TALK_FALLBACK)}
            onChange={(pushToTalkHotkey) => void update({ pushToTalkHotkey })}
          />
        </Row>
        <Row title="Hands-free mode" hint="Press once to start, press again to stop — no holding.">
          <GestureHotkey
            label="Hands-free mode"
            hotkey={settings.handsFreeHotkey}
            fallbackHint={formatAcceleratorMac(HANDS_FREE_FALLBACK)}
            onChange={(handsFreeHotkey) => void update({ handsFreeHotkey })}
          />
        </Row>
        <Row
          title="See changes"
          hint="Reveal a word-level diff of the last cleanup or polish. Empty disables it. Also editable on the Transform page."
        >
          <HotkeyRecorder
            value={settings.seeChangesHotkey.key}
            label="See changes"
            onChange={(accelerator) =>
              void update({ seeChangesHotkey: { kind: 'accelerator', key: accelerator } })
            }
          />
        </Row>
        <p className="row-hint">Press Esc while recording to cancel.</p>
      </section>

      <section className="card">
        <h2>Speech input</h2>
        <Row
          title="Microphone"
          hint="The input device to record from. System default follows your Mac's choice."
        >
          <select
            value={settings.inputDeviceName ?? ''}
            disabled={inputDevices.length === 0}
            onChange={(e) => void update({ inputDeviceName: e.target.value || null })}
          >
            <option value="">System default</option>
            {/* Keep a saved-but-absent device (e.g. unplugged) visible so the
                picker reflects the stored choice; recording falls back server-side. */}
            {settings.inputDeviceName && !inputDevices.includes(settings.inputDeviceName) && (
              <option value={settings.inputDeviceName}>{settings.inputDeviceName}</option>
            )}
            {inputDevices.map((name) => (
              <option key={name} value={name}>
                {name}
              </option>
            ))}
          </select>
        </Row>
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
    </div>
  );
}
