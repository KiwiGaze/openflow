import type { DictionaryEntry } from './types.js';

/** Trim and strip trailing slashes so `http://x:11434/` equals `http://x:11434`. */
export function normalizeBaseUrl(url: string): string {
  return url.trim().replace(/\/+$/, '');
}

export function isValidBaseUrl(url: string): boolean {
  const normalized = normalizeBaseUrl(url);
  try {
    const parsed = new URL(normalized);
    return parsed.protocol === 'http:' || parsed.protocol === 'https:';
  } catch {
    return false;
  }
}

/** Returns an error message, or null when the entry is valid. */
export function validateDictionaryEntry(
  entry: DictionaryEntry,
  existing: readonly DictionaryEntry[],
): string | null {
  const from = entry.from.trim();
  const to = entry.to.trim();
  if (from.length === 0) return 'The “heard as” text cannot be empty.';
  if (to.length === 0) return 'The replacement cannot be empty.';
  if (from.length > 100 || to.length > 100) return 'Entries are limited to 100 characters.';
  if (from.toLowerCase() === to.toLowerCase()) {
    return 'Replacement is identical to the heard text.';
  }
  const duplicate = existing.some((e) => e.from.trim().toLowerCase() === from.toLowerCase());
  if (duplicate) return `“${from}” is already in the dictionary.`;
  return null;
}
