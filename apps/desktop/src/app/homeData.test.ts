import { describe, expect, it } from 'vitest';
import type { HistoryEntry } from '@velata/core';
import { greetingForHour, groupHistoryByDay } from './homeData.js';

function entry(at: number, id = String(at)): HistoryEntry {
  return {
    id,
    at,
    text: 'final',
    rawText: 'raw',
    modeId: 'default',
    appName: null,
    durationMs: null,
    wordCount: 1,
    usedAi: false,
  };
}

/** Local-time epoch ms, so day grouping is timezone-independent in the test. */
function localMs(year: number, month1: number, day: number, hour = 12, minute = 0): number {
  return new Date(year, month1 - 1, day, hour, minute).getTime();
}

describe('greetingForHour', () => {
  it('returns morning before noon', () => {
    expect(greetingForHour(0)).toBe('Good morning');
    expect(greetingForHour(11)).toBe('Good morning');
  });

  it('switches to afternoon at noon', () => {
    expect(greetingForHour(12)).toBe('Good afternoon');
    expect(greetingForHour(17)).toBe('Good afternoon');
  });

  it('switches to evening at 18:00', () => {
    expect(greetingForHour(18)).toBe('Good evening');
    expect(greetingForHour(23)).toBe('Good evening');
  });
});

describe('groupHistoryByDay', () => {
  const now = new Date(2026, 5, 13, 14, 30); // 2026-06-13, local

  it('returns no groups for empty input', () => {
    expect(groupHistoryByDay([], now)).toEqual([]);
  });

  it('labels the current and previous local day', () => {
    const groups = groupHistoryByDay(
      [entry(localMs(2026, 6, 13, 9)), entry(localMs(2026, 6, 12, 22))],
      now,
    );
    expect(groups.map((g) => g.label)).toEqual(['Today', 'Yesterday']);
  });

  it('labels older same-year days as month and day without a year', () => {
    const groups = groupHistoryByDay([entry(localMs(2026, 6, 10))], now);
    expect(groups).toHaveLength(1);
    expect(groups[0]?.label).toBe(
      new Intl.DateTimeFormat(undefined, { month: 'long', day: 'numeric' }).format(
        new Date(2026, 5, 10),
      ),
    );
    expect(groups[0]?.label).not.toMatch(/2026/);
  });

  it('appends the year only when it differs from now', () => {
    const groups = groupHistoryByDay([entry(localMs(2025, 12, 31))], now);
    expect(groups[0]?.label).toContain('2025');
  });

  it('orders groups by first appearance (newest day first) and keeps incoming order within a day', () => {
    const first = entry(localMs(2026, 6, 13, 14), 'today-a');
    const second = entry(localMs(2026, 6, 13, 9), 'today-b');
    const older = entry(localMs(2026, 6, 11, 16), 'older');
    const groups = groupHistoryByDay([first, second, older], now);
    expect(groups.map((g) => g.label)).toEqual([
      'Today',
      new Intl.DateTimeFormat(undefined, { month: 'long', day: 'numeric' }).format(
        new Date(2026, 5, 11),
      ),
    ]);
    expect(groups[0]?.entries.map((e) => e.id)).toEqual(['today-a', 'today-b']);
    expect(groups[1]?.entries.map((e) => e.id)).toEqual(['older']);
  });

  it('groups entries from the same day together even when interleaved order would split them', () => {
    const groups = groupHistoryByDay(
      [
        entry(localMs(2026, 6, 13, 14), 'a'),
        entry(localMs(2026, 6, 12, 10), 'b'),
        entry(localMs(2026, 6, 13, 8), 'c'),
      ],
      now,
    );
    expect(groups).toHaveLength(2);
    expect(groups[0]?.entries.map((e) => e.id)).toEqual(['a', 'c']);
    expect(groups[1]?.entries.map((e) => e.id)).toEqual(['b']);
  });
});
