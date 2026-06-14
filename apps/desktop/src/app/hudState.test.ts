import { describe, expect, it } from 'vitest';
import { barScales, hudGlyph, hudLabel, hudVisible } from './hudState.js';

describe('hudLabel', () => {
  it('describes each stage', () => {
    expect(hudLabel({ status: 'recording', job: 'dictation', message: null })).toBe('Listening…');
    expect(hudLabel({ status: 'transcribing', job: 'dictation', message: null })).toBe(
      'Transcribing…',
    );
    expect(hudLabel({ status: 'error', job: null, message: 'mic missing' })).toBe('mic missing');
    expect(hudLabel({ status: 'idle', job: null, message: null })).toBe('');
  });

  it('distinguishes the polishing flows', () => {
    // The post-dictation transform shows a generic label.
    expect(hudLabel({ status: 'polishing', job: 'dictation', message: null })).toBe('Polishing…');
    // A prompt transform shows its name from the message, with a fallback.
    expect(hudLabel({ status: 'polishing', job: 'transform', message: 'Polish' })).toBe('Polish…');
    expect(hudLabel({ status: 'polishing', job: 'transform', message: null })).toBe(
      'Transforming…',
    );
  });

  it('quotes the inserted text on the success flash and ellipsizes long previews', () => {
    expect(hudLabel({ status: 'inserted', job: null, message: 'Ship it Friday.' })).toBe(
      '“Ship it Friday.”',
    );
    expect(hudLabel({ status: 'inserted', job: null, message: '' })).toBe('Inserted');
    const long = 'a'.repeat(80);
    const shown = hudLabel({ status: 'inserted', job: null, message: long });
    expect(shown.length).toBeLessThan(long.length);
    expect(shown.endsWith('…”')).toBe(true);
  });
});

describe('hudGlyph', () => {
  it('marks severity without relying on color', () => {
    expect(hudGlyph({ status: 'inserted', job: null, message: 'x' })).toBe('✓');
    expect(hudGlyph({ status: 'notice', job: null, message: 'x' })).toBe('ⓘ');
    expect(hudGlyph({ status: 'error', job: null, message: 'x' })).toBe('⚠');
    expect(hudGlyph({ status: 'recording', job: 'dictation', message: null })).toBe('');
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
