import { describe, expect, it } from 'vitest';
import type { Transform } from '@velata/core';
import {
  noteTitle,
  relativeTime,
  splitTransformBar,
  transformChips,
  versionLabel,
  type TransformChip,
} from './scratchpadView.js';

const transforms: Transform[] = [
  {
    id: 'prompt-engineer',
    name: 'Prompt Engineer',
    instruction: 'x',
    hotkey: '',
    builtIn: true,
  },
];

describe('noteTitle', () => {
  it('falls back to Untitled for blank titles', () => {
    expect(noteTitle('  Notes ')).toBe('Notes');
    expect(noteTitle('   ')).toBe('Untitled');
    expect(noteTitle('')).toBe('Untitled');
  });
});

describe('relativeTime', () => {
  const now = 1_000_000_000_000;
  it('reads recent times compactly', () => {
    expect(relativeTime(now, now)).toBe('just now');
    expect(relativeTime(now - 30_000, now)).toBe('just now');
    expect(relativeTime(now - 5 * 60_000, now)).toBe('5m ago');
    expect(relativeTime(now - 3 * 3_600_000, now)).toBe('3h ago');
    expect(relativeTime(now - 26 * 3_600_000, now)).toBe('yesterday');
    expect(relativeTime(now - 4 * 86_400_000, now)).toBe('4d ago');
  });
  it('falls back to a date for older times and never goes negative', () => {
    expect(relativeTime(now - 30 * 86_400_000, now)).toBe(
      new Date(now - 30 * 86_400_000).toLocaleDateString(),
    );
    // A future timestamp (clock skew) reads "just now", not a negative span.
    expect(relativeTime(now + 10_000, now)).toBe('just now');
  });
});

describe('versionLabel', () => {
  it('labels each source, naming the transform when known', () => {
    expect(versionLabel('created', null, transforms)).toBe('Created');
    expect(versionLabel('restore', null, transforms)).toBe('Before restore');
    expect(versionLabel('transform', null, transforms)).toBe('Before Polish');
    expect(versionLabel('transform', 'prompt-engineer', transforms)).toBe('Before Prompt Engineer');
    // An unknown transform id degrades to the generic Polish label.
    expect(versionLabel('transform', 'gone', transforms)).toBe('Before Polish');
  });
});

describe('transformChips', () => {
  it('puts Polish first, then each transform by name', () => {
    const chips = transformChips({ transforms } as never);
    expect(chips[0]).toEqual({ id: null, label: 'Polish' });
    expect(chips[1]).toEqual({ id: 'prompt-engineer', label: 'Prompt Engineer' });
  });
});

describe('splitTransformBar', () => {
  const chip = (id: string | null, label: string): TransformChip => ({ id, label });

  it('keeps Polish alone inline when there are no transforms', () => {
    const { visible, overflow } = splitTransformBar([chip(null, 'Polish')]);
    expect(visible).toEqual([chip(null, 'Polish')]);
    expect(overflow).toEqual([]);
  });

  it('keeps Polish plus the first 3 transforms inline, rest in overflow', () => {
    const chips = [
      chip(null, 'Polish'),
      chip('a', 'A'),
      chip('b', 'B'),
      chip('c', 'C'),
      chip('d', 'D'),
      chip('e', 'E'),
    ];
    const { visible, overflow } = splitTransformBar(chips);
    expect(visible).toEqual([chip(null, 'Polish'), chip('a', 'A'), chip('b', 'B'), chip('c', 'C')]);
    expect(overflow).toEqual([chip('d', 'D'), chip('e', 'E')]);
  });
});
