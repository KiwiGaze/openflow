import { describe, expect, it } from 'vitest';
import type { Snippet } from '@velata/core';
import { filterSnippets } from './snippetView.js';

const snippets: Snippet[] = [
  { trigger: 'my email', expansion: 'me@example.com', wholeUtterance: false },
  { trigger: 'sig', expansion: 'Best,\nAlex', wholeUtterance: true },
  { trigger: 'docs link', expansion: 'https://velata.app/docs', wholeUtterance: false },
];

describe('filterSnippets', () => {
  it('returns every snippet with its index for a blank query', () => {
    const result = filterSnippets(snippets, '');
    expect(result.map((r) => r.index)).toEqual([0, 1, 2]);
  });

  it('returns every snippet when the query is only whitespace', () => {
    expect(filterSnippets(snippets, '   ')).toHaveLength(3);
  });

  it('matches the trigger case-insensitively', () => {
    const result = filterSnippets(snippets, 'MY EMAIL');
    expect(result.map((r) => r.snippet.trigger)).toEqual(['my email']);
  });

  it('matches the expansion, not only the trigger', () => {
    const result = filterSnippets(snippets, 'example.com');
    expect(result).toHaveLength(1);
    expect(result[0]?.snippet.trigger).toBe('my email');
  });

  it('matches a query only reachable through the expansion, ignoring case', () => {
    const result = filterSnippets(snippets, 'alex');
    expect(result.map((r) => r.snippet.trigger)).toEqual(['sig']);
  });

  it('trims the query before matching', () => {
    const result = filterSnippets(snippets, '  docs  ');
    expect(result.map((r) => r.snippet.trigger)).toEqual(['docs link']);
  });

  it('preserves the original index so edits address the right row', () => {
    const result = filterSnippets(snippets, 'docs link');
    expect(result).toEqual([{ snippet: snippets[2], index: 2 }]);
  });

  it('returns nothing when no snippet matches', () => {
    expect(filterSnippets(snippets, 'nonexistent')).toEqual([]);
  });
});
