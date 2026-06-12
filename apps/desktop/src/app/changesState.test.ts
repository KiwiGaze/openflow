import { describe, expect, it } from 'vitest';
import type { TranscriptionResult } from '@openflow/core';
import { initialChangesState, nextChangesState } from './changesState.js';

const result: TranscriptionResult = {
  raw: 'the quick fox',
  original: 'the quick fox',
  text: 'the slow fox',
  modeId: 'polish',
  polished: true,
  durationMs: 100,
};

describe('nextChangesState', () => {
  it('shows the result on the first toggle', () => {
    const next = nextChangesState(initialChangesState, { type: 'toggle', result });
    expect(next).toEqual({ result, visible: true });
  });

  it('hides again on a second toggle, keeping the result', () => {
    const shown = nextChangesState(initialChangesState, { type: 'toggle', result });
    const hidden = nextChangesState(shown, { type: 'toggle', result });
    expect(hidden).toEqual({ result, visible: false });
  });

  it('hides when a new job starts', () => {
    const shown = nextChangesState(initialChangesState, { type: 'toggle', result });
    expect(nextChangesState(shown, { type: 'pipeline', status: 'recording' }).visible).toBe(false);
  });

  it('stays put while the pipeline is idle', () => {
    const shown = nextChangesState(initialChangesState, { type: 'toggle', result });
    expect(nextChangesState(shown, { type: 'pipeline', status: 'idle' })).toBe(shown);
  });

  it('hides on close', () => {
    const shown = nextChangesState(initialChangesState, { type: 'toggle', result });
    expect(nextChangesState(shown, { type: 'close' }).visible).toBe(false);
  });
});
