import { useEffect, useState, type JSX } from 'react';
import { formatAcceleratorMac, LANGUAGES, PUSH_TO_TALK_FALLBACK, type Hotkey } from '@velata/core';
import { usePermissions, type ModelsApi, type SettingsApi } from '../hooks.js';
import { ipc } from '../ipc.js';
import { Callout } from '../components/Callout.js';
import { HotkeyRecorder } from '../components/HotkeyRecorder.js';
import { Row } from '../components/Row.js';

const FN_PUSH_TO_TALK: Hotkey = { kind: 'hold', key: 'fn' };

/**
 * Push-to-talk trigger control (the right-hand column of its row). The fn-key
 * gesture is the default; a user can instead bind a literal accelerator, and
 * restore the fn gesture from one (the gesture→accelerator transition used to be
 * one-way). The Input-Monitoring grant flow lives in a section-level callout, not
 * here, because that is how callouts read across the settings tabs.
 */
function PushToTalkHotkey({
  hotkey,
  onChange,
}: {
  hotkey: Hotkey;
  onChange: (next: Hotkey) => void;
}): JSX.Element {
  if (hotkey.kind !== 'accelerator') {
    return (
      <div className="hotkey-gesture">
        <span className="hotkey-gesture-current">Hold 🌐 fn</span>
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
        label="Push to talk"
        emptyLabel="Set…"
        onChange={(accelerator) => {
          onChange({ kind: 'accelerator', key: accelerator });
        }}
      />
      {/* Push-to-talk should stay bound, so the way back is "use the fn key",
          not a Clear. Setting the fn gesture surfaces the Input-Monitoring
          callout below when the permission is not granted. */}
      <button
        type="button"
        className="btn btn-quiet"
        onClick={() => {
          onChange(FN_PUSH_TO_TALK);
        }}
      >
        Use the fn key
      </button>
    </div>
  );
}

/**
 * Optional separate accelerator to toggle hands-free. The primary way to latch
 * hands-free is a quick tap of the push-to-talk key (see the row hint); this is
 * an extra, always-disable-able shortcut, so it offers a Clear that PTT does not.
 */
function HandsFreeHotkey({
  hotkey,
  onChange,
}: {
  hotkey: Hotkey;
  onChange: (next: Hotkey) => void;
}): JSX.Element {
  return (
    <div className="hotkey-gesture">
      <HotkeyRecorder
        value={hotkey.key}
        label="Hands-free mode"
        emptyLabel="Not set"
        onChange={(accelerator) => {
          onChange({ kind: 'accelerator', key: accelerator });
        }}
      />
      {hotkey.key.trim() !== '' && (
        <button
          type="button"
          className="btn btn-quiet"
          aria-label="Clear hands-free shortcut"
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

  // Poll permissions so the Input-Monitoring callout clears once the user grants
  // it in System Settings (same pattern Onboarding uses). The grant still needs
  // a relaunch to take effect, which the callout copy states.
  const permissions = usePermissions();
  // Show the grant flow only when push-to-talk actually relies on observing fn
  // (the gesture trigger) and the permission is not yet granted. An accelerator
  // push-to-talk does not need Input Monitoring.
  const needsInputMonitoring =
    settings.pushToTalkHotkey.kind !== 'accelerator' &&
    permissions !== null &&
    permissions.inputMonitoring !== 'granted';

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
          <PushToTalkHotkey
            hotkey={settings.pushToTalkHotkey}
            onChange={(pushToTalkHotkey) => void update({ pushToTalkHotkey })}
          />
        </Row>
        {needsInputMonitoring && (
          <Callout
            variant="warn"
            action={{
              label: 'Grant Input Monitoring',
              onClick: () => void ipc.requestInputMonitoring(),
            }}
          >
            The fn key needs Input Monitoring. Until you grant it, push-to-talk uses{' '}
            {formatAcceleratorMac(PUSH_TO_TALK_FALLBACK)}. The grant takes effect after you relaunch
            Velata.
          </Callout>
        )}
        <Row
          title="Hands-free mode"
          hint="Quick-tap your push-to-talk key to latch hands-free (tap again to stop). Or set a separate shortcut."
        >
          <HandsFreeHotkey
            hotkey={settings.handsFreeHotkey}
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
