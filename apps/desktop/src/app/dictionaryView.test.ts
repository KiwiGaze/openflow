import { describe, expect, it } from 'vitest';
import type { DictionaryEntry } from '@velata/core';
import { entryKind, filterDictionary } from './dictionaryView.js';

const entries: DictionaryEntry[] = [
  { from: 'open flow', to: 'Velata' },
  { from: 'Velata', to: 'Velata' },
  { from: 'kubernetes', to: 'Kubernetes' },
];

describe('entryKind', () => {
  it('treats equal from/to as vocabulary', () => {
    expect(entryKind({ from: 'Velata', to: 'Velata' })).toBe('vocabulary');
  });

  it('treats differing from/to as a correction', () => {
    expect(entryKind({ from: 'open flow', to: 'Velata' })).toBe('correction');
  });

  it('ignores surrounding whitespace when comparing', () => {
    expect(entryKind({ from: '  term ', to: 'term' })).toBe('vocabulary');
  });
});

describe('filterDictionary', () => {
  it('returns every entry with its index for a blank query', () => {
    const result = filterDictionary(entries, '');
    expect(result.map((r) => r.index)).toEqual([0, 1, 2]);
  });

  it('returns every entry when the query is only whitespace', () => {
    expect(filterDictionary(entries, '   ')).toHaveLength(3);
  });

  it('matches case-insensitively', () => {
    const result = filterDictionary(entries, 'VELATA');
    expect(result.map((r) => r.entry.from)).toEqual(['open flow', 'Velata']);
  });

  it('matches the replacement (to), not only the heard text (from)', () => {
    const result = filterDictionary([{ from: 'open flow', to: 'Velata' }], 'velata');
    expect(result).toHaveLength(1);
    expect(result[0]?.entry.to).toBe('Velata');
  });

  it('trims the query before matching', () => {
    const result = filterDictionary(entries, '  flow  ');
    expect(result.map((r) => r.entry.from)).toEqual(['open flow']);
  });

  it('preserves the original index so edits address the right row', () => {
    const result = filterDictionary(entries, 'Kubernetes');
    expect(result).toEqual([{ entry: entries[2], index: 2 }]);
  });

  it('returns nothing when no entry matches', () => {
    expect(filterDictionary(entries, 'nonexistent')).toEqual([]);
  });
});
