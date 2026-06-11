import type { PipelineState } from '@openflow/core';

/** Longest inserted-text preview shown in the success flash. */
const MAX_PREVIEW = 48;

function ellipsize(text: string, max = MAX_PREVIEW): string {
  const trimmed = text.trim();
  return trimmed.length > max ? `${trimmed.slice(0, max - 1)}…` : trimmed;
}

/** Text shown in the HUD pill for each pipeline state. */
export function hudLabel(state: PipelineState): string {
  switch (state.status) {
    case 'recording':
      if (state.job === 'refineSelection') return 'Listening for instruction…';
      // Dictation names the active mode (07 §5); Rewrite is an action, not a mode.
      return state.message ? `Listening — ${state.message}` : 'Listening…';
    case 'transcribing':
      return 'Transcribing…';
    case 'refining':
      if (state.job === 'refineSelection') return 'Rewriting…';
      if (state.job === 'polishSelection') return 'Polishing selection…';
      return 'Cleaning up…';
    case 'inserting':
      return 'Inserting…';
    case 'inserted':
      return state.message ? `“${ellipsize(state.message)}”` : 'Inserted';
    case 'notice':
    case 'error':
      return state.message ?? 'Something went wrong — your text is on the clipboard';
    case 'idle':
      return '';
  }
}

/**
 * Leading severity glyph so meaning survives without color (UX-34). It is
 * `aria-hidden`; the label text carries the meaning for assistive tech.
 */
export function hudGlyph(state: PipelineState): string {
  switch (state.status) {
    case 'inserted':
      return '✓';
    case 'notice':
      return 'ⓘ';
    case 'error':
      return '⚠';
    default:
      return '';
  }
}

export function hudVisible(state: PipelineState): boolean {
  return state.status !== 'idle';
}

/** Per-bar scale factors for the level meter, mid bars move the most. */
export function barScales(level: number, bars = 5): number[] {
  const emphasis = [0.55, 0.8, 1, 0.8, 0.55];
  // Microphone RMS for speech is roughly 0.01–0.2; expand into a visible range.
  const boosted = Math.min(1, level * 9);
  return Array.from({ length: bars }, (_, i) => {
    const factor = emphasis[i % emphasis.length] ?? 1;
    return Math.max(0.18, boosted * factor);
  });
}
