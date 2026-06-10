import type { PipelineState } from '@openflow/core';

/** Text shown in the HUD pill for each pipeline state. */
export function hudLabel(state: PipelineState): string {
  switch (state.status) {
    case 'recording':
      return state.job === 'refineSelection' ? 'Listening for instruction…' : 'Listening…';
    case 'transcribing':
      return 'Transcribing…';
    case 'refining':
      return 'Polishing…';
    case 'inserting':
      return 'Inserting…';
    case 'notice':
    case 'error':
      return state.message ?? 'Something went wrong';
    case 'idle':
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
