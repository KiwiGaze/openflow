import type { JSX } from 'react';

export function ScratchpadTab(): JSX.Element {
  return (
    <div className="tab-body">
      <h1>Scratchpad</h1>
      <section className="card">
        <p className="row-hint">A quiet place to draft and tidy text by voice.</p>
      </section>
    </div>
  );
}
