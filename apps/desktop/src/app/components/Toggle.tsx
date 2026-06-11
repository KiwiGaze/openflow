import type { JSX } from 'react';

interface Props {
  checked: boolean;
  onChange: (checked: boolean) => void;
  /** Required so every switch has a real accessible name (UX-30). */
  label: string;
}

export function Toggle({ checked, onChange, label }: Props): JSX.Element {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      className={`toggle ${checked ? 'toggle-on' : ''}`}
      onClick={() => {
        onChange(!checked);
      }}
    >
      <span className="toggle-knob" />
    </button>
  );
}
