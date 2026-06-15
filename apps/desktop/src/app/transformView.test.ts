import { describe, expect, it } from 'vitest';
import type { Prompt } from '@velata/core';
import { filterPrompts } from './transformView.js';

const prompts: Prompt[] = [
  {
    id: 'a',
    name: 'Polish',
    instruction: 'Fix grammar and spelling.',
    shortcut: 'Alt+Shift+P',
    builtIn: true,
  },
  {
    id: 'b',
    name: 'Concise',
    instruction: 'Tighten the wording.',
    shortcut: 'Alt+1',
    builtIn: false,
  },
  {
    id: 'c',
    name: 'Bullet points',
    instruction: 'Restructure into bullets.',
    shortcut: '',
    builtIn: false,
  },
];

describe('filterPrompts', () => {
  it('returns every prompt in order for a blank query', () => {
    expect(filterPrompts(prompts, '').map((p) => p.id)).toEqual(['a', 'b', 'c']);
  });

  it('returns every prompt when the query is only whitespace', () => {
    expect(filterPrompts(prompts, '   ')).toHaveLength(3);
  });

  it('matches the name case-insensitively', () => {
    expect(filterPrompts(prompts, 'CONCISE').map((p) => p.id)).toEqual(['b']);
  });

  it('matches the instruction, not only the name', () => {
    expect(filterPrompts(prompts, 'bullets').map((p) => p.id)).toEqual(['c']);
  });

  it('trims the query before matching', () => {
    expect(filterPrompts(prompts, '  spelling  ').map((p) => p.id)).toEqual(['a']);
  });

  it('returns nothing when no prompt matches', () => {
    expect(filterPrompts(prompts, 'nonexistent')).toEqual([]);
  });

  it('does not mutate or alias the input array', () => {
    const result = filterPrompts(prompts, '');
    expect(result).not.toBe(prompts);
  });
});
