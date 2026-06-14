import { describe, expect, it } from 'vitest';
import type { Transform } from '@velata/core';
import { filterTransforms } from './transformView.js';

const transforms: Transform[] = [
  {
    id: 'a',
    name: 'Prompt Engineer',
    instruction: 'Rewrite as a clear prompt.',
    hotkey: '',
    builtIn: true,
  },
  {
    id: 'b',
    name: 'Concise',
    instruction: 'Tighten the wording.',
    hotkey: 'Alt+1',
    builtIn: false,
  },
  {
    id: 'c',
    name: 'Bullet points',
    instruction: 'Restructure into bullets.',
    hotkey: '',
    builtIn: false,
  },
];

describe('filterTransforms', () => {
  it('returns every transform in order for a blank query', () => {
    expect(filterTransforms(transforms, '').map((t) => t.id)).toEqual(['a', 'b', 'c']);
  });

  it('returns every transform when the query is only whitespace', () => {
    expect(filterTransforms(transforms, '   ')).toHaveLength(3);
  });

  it('matches the name case-insensitively', () => {
    expect(filterTransforms(transforms, 'CONCISE').map((t) => t.id)).toEqual(['b']);
  });

  it('matches the instruction, not only the name', () => {
    expect(filterTransforms(transforms, 'bullets').map((t) => t.id)).toEqual(['c']);
  });

  it('trims the query before matching', () => {
    expect(filterTransforms(transforms, '  prompt  ').map((t) => t.id)).toEqual(['a']);
  });

  it('returns nothing when no transform matches', () => {
    expect(filterTransforms(transforms, 'nonexistent')).toEqual([]);
  });

  it('does not mutate or alias the input array', () => {
    const result = filterTransforms(transforms, '');
    expect(result).not.toBe(transforms);
  });
});
