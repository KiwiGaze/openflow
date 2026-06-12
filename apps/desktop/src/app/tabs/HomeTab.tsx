import type { JSX } from 'react';

export function HomeTab(): JSX.Element {
  return (
    <div className="tab-body">
      <h1>Home</h1>
      <section className="card">
        <p className="row-hint">Your dictation history will appear here.</p>
      </section>
    </div>
  );
}
