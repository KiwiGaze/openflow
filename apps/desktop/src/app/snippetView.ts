/** Pure presentation logic for the Snippets tab: search filtering. */

import type { Snippet } from '@velata/core';

/** A saved snippet paired with its index in the unfiltered list, so edit and
 * delete still address the right row after a search narrows the list. */
export interface IndexedSnippet {
  snippet: Snippet;
  index: number;
}

/**
 * Filters snippets by a search query, matching case-insensitively against both
 * `trigger` and `expansion`. A blank query (after trimming) returns every
 * snippet. Each result keeps its original index for stable edit/delete.
 */
export function filterSnippets(snippets: readonly Snippet[], query: string): IndexedSnippet[] {
  const needle = query.trim().toLowerCase();
  const indexed = snippets.map((snippet, index) => ({ snippet, index }));
  if (needle === '') return indexed;
  return indexed.filter(
    ({ snippet }) =>
      snippet.trigger.toLowerCase().includes(needle) ||
      snippet.expansion.toLowerCase().includes(needle),
  );
}
