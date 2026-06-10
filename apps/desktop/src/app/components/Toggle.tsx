import type { JSX } from 'react';

interface Props {
  checked: boolean;
  onChange: (checked: boolean) => void;
  label?: string;
}

export function Toggle({ checked, onChange, label }: Props): JSX.Element {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label ?? 'toggle'}
      className={`toggle ${checked ? 'toggle-on' : ''}`}
      onClick={() => {
        onChange(!checked);
      }}
    >
      <span className="toggle-knob" />
    </button>
  );
}
