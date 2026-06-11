import type { DictionaryEntry, Snippet } from './types.js';

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

const LOCAL_HOSTS = new Set(['localhost', '127.0.0.1', '::1', '[::1]', '0.0.0.0']);

/** True when the endpoint host is loopback — refined text stays on-device. */
export function isLocalEndpoint(url: string): boolean {
  try {
    return LOCAL_HOSTS.has(new URL(normalizeBaseUrl(url)).hostname);
  } catch {
    return false;
  }
}

/**
 * True when `from` already matches an entry's "heard as" text
 * (case-insensitive). Shared by the editor validation and the suggestion
 * accept flow so the duplicate rule cannot drift.
 */
export function hasDictionaryEntry(from: string, existing: readonly DictionaryEntry[]): boolean {
  const needle = from.trim().toLowerCase();
  return existing.some((e) => e.from.trim().toLowerCase() === needle);
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
  if (hasDictionaryEntry(from, existing)) return `“${from}” is already in the dictionary.`;
  return null;
}

/** Max characters for a snippet expansion — long enough for canned replies. */
export const MAX_SNIPPET_EXPANSION = 4000;

/** Returns an error message, or null when the snippet is valid. */
export function validateSnippet(snippet: Snippet, existing: readonly Snippet[]): string | null {
  const trigger = snippet.trigger.trim();
  if (trigger.length === 0) return 'The trigger phrase cannot be empty.';
  if (snippet.expansion.length === 0) return 'The expansion cannot be empty.';
  if (trigger.length > 100) return 'Triggers are limited to 100 characters.';
  if (snippet.expansion.length > MAX_SNIPPET_EXPANSION) {
    return `Expansions are limited to ${String(MAX_SNIPPET_EXPANSION)} characters.`;
  }
  const duplicate = existing.some((s) => s.trigger.trim().toLowerCase() === trigger.toLowerCase());
  if (duplicate) return `“${trigger}” is already a snippet.`;
  return null;
}
