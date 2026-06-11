import { useEffect, useState, type JSX } from 'react';
import type { PipelineState } from '@openflow/core';
import { events, subscribe } from './ipc.js';
import { barScales, hudGlyph, hudLabel, hudVisible } from './hudState.js';

/**
 * The always-running overlay. The window itself never hides (see hud.rs);
 * content fades in and out with pipeline state instead.
 */
export function Hud(): JSX.Element {
  const [state, setState] = useState<PipelineState>({ status: 'idle', job: null, message: null });
  const [level, setLevel] = useState(0);

  useEffect(() => {
    const cleanups = [
      subscribe(events.onPipelineState(setState)),
      subscribe(events.onAudioLevel(setLevel)),
    ];
    return () => {
      cleanups.forEach((fn) => {
        fn();
      });
    };
  }, []);

  const visible = hudVisible(state);
  const recording = state.status === 'recording';
  const busy =
    state.status === 'transcribing' || state.status === 'refining' || state.status === 'inserting';
  const glyph = hudGlyph(state);

  return (
    <div className={`hud-pill ${visible ? 'hud-visible' : ''} hud-${state.status}`}>
      {recording && (
        <div className="hud-bars" aria-hidden>
          {barScales(level).map((scale, i) => (
            <span key={i} style={{ transform: `scaleY(${scale})` }} />
          ))}
        </div>
      )}
      {busy && <span className="hud-spinner" aria-hidden />}
      {glyph && (
        <span
          className={`hud-glyph ${state.status === 'inserted' ? 'hud-glyph-ok' : ''}`}
          aria-hidden
        >
          {glyph}
        </span>
      )}
      <span className="hud-label" aria-live="polite" aria-atomic="true">
        {hudLabel(state)}
      </span>
    </div>
  );
}
