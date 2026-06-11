import { describe, expect, it } from 'vitest';
import { barScales, hudLabel, hudVisible } from './hudState.js';

describe('hudLabel', () => {
  it('describes each stage', () => {
    expect(hudLabel({ status: 'recording', job: 'dictation', message: null })).toBe('Listening…');
    expect(hudLabel({ status: 'recording', job: 'refineSelection', message: null })).toBe(
      'Listening for instruction…',
    );
    expect(hudLabel({ status: 'transcribing', job: 'dictation', message: null })).toBe(
      'Transcribing…',
    );
    expect(hudLabel({ status: 'error', job: null, message: 'mic missing' })).toBe('mic missing');
    expect(hudLabel({ status: 'idle', job: null, message: null })).toBe('');
  });

  it('distinguishes the refining flows', () => {
    expect(hudLabel({ status: 'refining', job: 'dictation', message: null })).toBe('Polishing…');
    expect(hudLabel({ status: 'refining', job: 'refineSelection', message: null })).toBe(
      'Rewriting…',
    );
    expect(hudLabel({ status: 'refining', job: 'polishSelection', message: null })).toBe(
      'Polishing selection…',
    );
    // Transforms show their name from the message, with a generic fallback.
    expect(hudLabel({ status: 'refining', job: 'transform', message: 'Concise' })).toBe('Concise…');
    expect(hudLabel({ status: 'refining', job: 'transform', message: null })).toBe('Transforming…');
  });
});

describe('hudVisible', () => {
  it('hides only when idle', () => {
    expect(hudVisible({ status: 'idle', job: null, message: null })).toBe(false);
    expect(hudVisible({ status: 'notice', job: null, message: 'hi' })).toBe(true);
  });
});

describe('barScales', () => {
  it('clamps to a visible minimum and a maximum of 1', () => {
    const silent = barScales(0);
    expect(silent.every((s) => s >= 0.18)).toBe(true);
    const loud = barScales(1);
    expect(Math.max(...loud)).toBeLessThanOrEqual(1);
    expect(loud).toHaveLength(5);
  });

  it('middle bar moves the most', () => {
    const scales = barScales(0.08);
    expect(scales[2]).toBeGreaterThan(scales[0] ?? 0);
  });
});
