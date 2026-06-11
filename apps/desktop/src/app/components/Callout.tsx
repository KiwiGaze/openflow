import type { JSX, ReactNode } from 'react';

interface Props {
  /** `info` (accent bar) for tips, `warn` (amber) for actionable recovery. */
  variant?: 'info' | 'warn';
  children: ReactNode;
  /** Optional inline action, e.g. "Open System Settings". */
  action?: { label: string; onClick: () => void };
  /** When set, renders a dismiss button. */
  onDismiss?: () => void;
}

/** Severity glyph; aria-hidden, so meaning rides the message text. */
const GLYPH: Record<NonNullable<Props['variant']>, string> = {
  info: 'ⓘ',
  warn: '⚠',
};

/**
 * A non-modal inline notice (09 §5.8): no-model recovery, permission-off
 * callouts, and feature tips. Generalizes the existing `.privacy-note`
 * left-bar idiom — not new machinery.
 */
export function Callout({ variant = 'info', children, action, onDismiss }: Props): JSX.Element {
  return (
    <aside className={`callout callout-${variant}`} role="note">
      <span className="callout-icon" aria-hidden>
        {GLYPH[variant]}
      </span>
      <div className="callout-body">
        <p>{children}</p>
        {action && (
          <button className="btn btn-sm" onClick={action.onClick}>
            {action.label}
          </button>
        )}
      </div>
      {onDismiss && (
        <button className="btn-quiet callout-dismiss" aria-label="Dismiss" onClick={onDismiss}>
          ×
        </button>
      )}
    </aside>
  );
}
