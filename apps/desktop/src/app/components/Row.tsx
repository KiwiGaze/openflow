import type { JSX, ReactNode } from 'react';

interface Props {
  title: string;
  hint?: string;
  children: ReactNode;
}

/** A settings row: label + hint on the left, control on the right. */
export function Row({ title, hint, children }: Props): JSX.Element {
  return (
    <div className="row">
      <div className="row-text">
        <div className="row-title">{title}</div>
        {hint && <div className="row-hint">{hint}</div>}
      </div>
      <div className="row-control">{children}</div>
    </div>
  );
}
