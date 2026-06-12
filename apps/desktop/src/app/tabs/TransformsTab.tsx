import type { JSX } from 'react';

export function TransformsTab(): JSX.Element {
  return (
    <div className="tab-body">
      <h1>Transforms</h1>
      <section className="card">
        <p className="row-hint">Reshape selected text with your own polish prompts.</p>
      </section>
    </div>
  );
}
