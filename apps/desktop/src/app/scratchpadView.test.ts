import { describe, expect, it } from 'vitest';
import type { Prompt } from '@velata/core';
import {
  noteTitle,
  relativeTime,
  splitTransformBar,
  transformChips,
  versionLabel,
  type TransformChip,
} from './scratchpadView.js';

const prompts: Prompt[] = [
  {
    id: 'polish',
    name: 'Polish',
    instruction: 'x',
    shortcut: 'Alt+Shift+P',
    builtIn: true,
  },
  {
    id: 'concise',
    name: 'Concise',
    instruction: 'y',
    shortcut: '',
    builtIn: false,
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
  it('labels each source, naming the prompt when known', () => {
    expect(versionLabel('created', null, prompts)).toBe('Created');
    expect(versionLabel('restore', null, prompts)).toBe('Before restore');
    expect(versionLabel('transform', null, prompts)).toBe('Before Polish');
    expect(versionLabel('transform', 'concise', prompts)).toBe('Before Concise');
    // An unknown prompt id degrades to the generic Polish label.
    expect(versionLabel('transform', 'gone', prompts)).toBe('Before Polish');
  });
});

describe('transformChips', () => {
  it('puts the synthetic Polish chip first, then each custom prompt by name', () => {
    const chips = transformChips({ prompts } as never);
    // The built-in Polish prompt is mapped to the null chip, never listed twice.
    expect(chips).toEqual([
      { id: null, label: 'Polish' },
      { id: 'concise', label: 'Concise' },
    ]);
  });
});

describe('splitTransformBar', () => {
  const chip = (id: string | null, label: string): TransformChip => ({ id, label });

  it('keeps Polish alone inline when there are no custom prompts', () => {
    const { visible, overflow } = splitTransformBar([chip(null, 'Polish')]);
    expect(visible).toEqual([chip(null, 'Polish')]);
    expect(overflow).toEqual([]);
  });

  it('keeps Polish plus the first 3 prompts inline, rest in overflow', () => {
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
