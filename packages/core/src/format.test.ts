import { describe, expect, it } from 'vitest';
import { formatBytes, formatDurationMs, formatProgress, todayIso } from './format.js';

describe('formatBytes', () => {
  it('formats common sizes', () => {
    expect(formatBytes(0)).toBe('0 B');
    expect(formatBytes(999)).toBe('999 B');
    expect(formatBytes(148_000_000)).toBe('148 MB');
    expect(formatBytes(77_700_000)).toBe('77.7 MB');
    expect(formatBytes(1_624_000_000)).toBe('1.6 GB');
  });

  it('handles invalid input', () => {
    expect(formatBytes(-1)).toBe('—');
    expect(formatBytes(Number.NaN)).toBe('—');
  });
});

describe('formatDurationMs', () => {
  it('formats sub-second, seconds and minutes', () => {
    expect(formatDurationMs(450)).toBe('450ms');
    expect(formatDurationMs(1234)).toBe('1.2s');
    expect(formatDurationMs(75_000)).toBe('1m 15s');
  });
});

describe('formatProgress', () => {
  it('formats percentages and caps at 100', () => {
    expect(formatProgress(50, 200)).toBe('25%');
    expect(formatProgress(250, 200)).toBe('100%');
    expect(formatProgress(10, 0)).toBe('—');
  });
});

describe('todayIso', () => {
  it('returns a YYYY-MM-DD date', () => {
    expect(todayIso()).toMatch(/^\d{4}-\d{2}-\d{2}$/);
  });
});
