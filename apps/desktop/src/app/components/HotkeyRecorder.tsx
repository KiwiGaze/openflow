import { useEffect, useId, useState, type JSX } from 'react';
import { acceleratorFromKeyboardEvent, formatAcceleratorMac } from '@velata/core';

interface Props {
  value: string;
  onChange: (accelerator: string) => void;
  /** What this shortcut controls, for the accessible name, e.g. "Dictation". */
  label?: string;
}

/** Click, then press the desired combo. Esc or Tab cancels (no focus trap). */
export function HotkeyRecorder({ value, onChange, label }: Props): JSX.Element {
  const [recording, setRecording] = useState(false);
  const [announce, setAnnounce] = useState('');
  const helperId = useId();

  useEffect(() => {
    if (!recording) return;
    const onKeyDown = (ev: KeyboardEvent): void => {
      // Tab must escape the recorder so keyboard focus is never trapped — let
      // the browser move focus instead of swallowing the key (UX-03).
      if (ev.code === 'Tab') {
        setRecording(false);
        setAnnounce('Cancelled');
        return;
      }
      ev.preventDefault();
      ev.stopPropagation();
      if (ev.code === 'Escape') {
        setRecording(false);
        setAnnounce('Cancelled');
        return;
      }
      const accelerator = acceleratorFromKeyboardEvent(ev);
      if (accelerator) {
        setRecording(false);
        setAnnounce(`Recorded ${formatAcceleratorMac(accelerator)}`);
        onChange(accelerator);
      }
    };
    const onBlur = (): void => {
      setRecording(false);
    };
    window.addEventListener('keydown', onKeyDown, true);
    window.addEventListener('blur', onBlur);
    return () => {
      window.removeEventListener('keydown', onKeyDown, true);
      window.removeEventListener('blur', onBlur);
    };
  }, [recording, onChange]);

  const name = label ? `${label} shortcut` : 'Shortcut';
  const formatted = formatAcceleratorMac(value);
  const display = recording ? 'Press shortcut…' : formatted === '' ? 'Set…' : formatted;

  return (
    <div className="hotkey-recorder">
      <button
        type="button"
        className={`hotkey-chip ${recording ? 'hotkey-recording' : ''}`}
        aria-label={`${name}, currently ${formatted === '' ? 'unset' : formatted}. Activate to record a new one.`}
        aria-describedby={recording ? helperId : undefined}
        onClick={() => {
          setRecording(true);
          setAnnounce('Recording — press a shortcut, or Esc to cancel.');
        }}
      >
        {display}
      </button>
      {recording && (
        <span id={helperId} className="hotkey-help">
          Press a shortcut, or Esc to cancel.
        </span>
      )}
      <span className="visually-hidden" aria-live="assertive">
        {announce}
      </span>
    </div>
  );
}
