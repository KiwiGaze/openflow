import type { JSX } from 'react';

export function StyleTab(): JSX.Element {
  return (
    <div className="tab-body">
      <h1>Style</h1>
      <section className="card">
        <p className="row-hint">Tune how your dictated text reads and sounds.</p>
      </section>
    </div>
  );
}
