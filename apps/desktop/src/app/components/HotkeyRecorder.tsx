import { useEffect, useRef, useState, type JSX } from 'react';
import { acceleratorFromKeyboardEvent, formatAcceleratorMac } from '@openflow/core';

interface Props {
  value: string;
  onChange: (accelerator: string) => void;
}

/** Click, then press the desired combo. Esc cancels. */
export function HotkeyRecorder({ value, onChange }: Props): JSX.Element {
  const [recording, setRecording] = useState(false);
  const buttonRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (!recording) return;
    const onKeyDown = (ev: KeyboardEvent): void => {
      ev.preventDefault();
      ev.stopPropagation();
      if (ev.code === 'Escape') {
        setRecording(false);
        return;
      }
      const accelerator = acceleratorFromKeyboardEvent(ev);
      if (accelerator) {
        setRecording(false);
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

  return (
    <button
      ref={buttonRef}
      type="button"
      className={`hotkey-chip ${recording ? 'hotkey-recording' : ''}`}
      onClick={() => {
        setRecording(true);
      }}
      title="Click, then press the new shortcut. Esc to cancel."
    >
      {recording ? 'Press shortcut…' : formatAcceleratorMac(value)}
    </button>
  );
}
