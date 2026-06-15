/** Pure presentation logic for the Dictionary tab: filtering and entry kind. */

import type { DictionaryEntry } from '@velata/core';

/**
 * What an entry does. A `vocabulary` entry has `from === to`: it biases speech
 * recognition toward the term and is listed to the AI without rewriting
 * anything ("kept as-is"). A `correction` entry rewrites `from` into `to`.
 */
export type EntryKind = 'vocabulary' | 'correction';

/** Classifies an entry by comparing its trimmed `from` and `to`. */
export function entryKind(entry: DictionaryEntry): EntryKind {
  return entry.from.trim() === entry.to.trim() ? 'vocabulary' : 'correction';
}

/** A saved entry paired with its index in the unfiltered dictionary, so edit
 * and delete still address the right row after a search narrows the list. */
export interface IndexedEntry {
  entry: DictionaryEntry;
  index: number;
}

/**
 * Filters dictionary entries by a search query, matching case-insensitively
 * against both `from` and `to`. A blank query (after trimming) returns every
 * entry. Each result keeps its original index for stable edit/delete.
 */
export function filterDictionary(
  entries: readonly DictionaryEntry[],
  query: string,
): IndexedEntry[] {
  const needle = query.trim().toLowerCase();
  const indexed = entries.map((entry, index) => ({ entry, index }));
  if (needle === '') return indexed;
  return indexed.filter(
    ({ entry }) =>
      entry.from.toLowerCase().includes(needle) || entry.to.toLowerCase().includes(needle),
  );
}
