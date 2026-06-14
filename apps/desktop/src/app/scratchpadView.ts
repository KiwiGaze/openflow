import type { Prompt, Settings } from '@velata/core';

/** A note's display title: the trimmed title, or "Untitled" when empty. */
export function noteTitle(title: string): string {
  const trimmed = title.trim();
  return trimmed.length > 0 ? trimmed : 'Untitled';
}

/**
 * A compact relative time for note rows and version entries. Recent times read
 * "just now" / "5m ago" / "3h ago" / "yesterday" / "4d ago"; anything older
 * falls back to a locale date. `now` is injected so the mapping is testable.
 */
export function relativeTime(ms: number, now: number): string {
  const diff = now - ms;
  if (diff < 0) return 'just now';
  const minute = 60_000;
  const hour = 60 * minute;
  const day = 24 * hour;
  if (diff < minute) return 'just now';
  if (diff < hour) return `${Math.floor(diff / minute)}m ago`;
  if (diff < day) return `${Math.floor(diff / hour)}h ago`;
  if (diff < 2 * day) return 'yesterday';
  if (diff < 7 * day) return `${Math.floor(diff / day)}d ago`;
  return new Date(ms).toLocaleDateString();
}

/** A human label for a version's `source` (with the prompt name when known). */
export function versionLabel(
  source: string,
  transformId: string | null,
  prompts: Prompt[],
): string {
  if (source === 'created') return 'Created';
  if (source === 'restore') return 'Before restore';
  if (source === 'transform') {
    const name = transformId ? prompts.find((p) => p.id === transformId)?.name : undefined;
    return name ? `Before ${name}` : 'Before Polish';
  }
  return source;
}

/** One transform chip: the built-in Polish (null id) plus each custom prompt. */
export interface TransformChip {
  /** Null for Polish (the server resolves the built-in Polish instruction). */
  id: string | null;
  label: string;
}

/**
 * The chip row shown above the editor: Polish first (the synthetic null-id
 * chip), then each custom prompt by name. The built-in Polish prompt is mapped
 * to the null chip — never listed twice — so its null `transformId` keeps
 * matching `transform_note_text`'s Polish branch and `versionLabel`'s "Before
 * Polish".
 */
export function transformChips(settings: Settings): TransformChip[] {
  return [
    { id: null, label: 'Polish' },
    ...settings.prompts.filter((p) => !p.builtIn).map((p) => ({ id: p.id, label: p.name })),
  ];
}

/** The transform bar split into inline chips and an overflow ("⋯ More") group. */
export interface TransformBar {
  /** Always shown: Polish plus the first 3 custom prompts (creation order). */
  visible: TransformChip[];
  /** Any remaining prompts, behind the "⋯ More" menu (creation order). */
  overflow: TransformChip[];
}

/**
 * Splits the full chip list for the single-note window's bottom bar: Polish and
 * the first 3 prompts stay inline, the rest move into an overflow menu. The full
 * list is kept elsewhere (version-history labels resolve every prompt id from
 * it), so this is a presentation-only split.
 */
export function splitTransformBar(chips: TransformChip[]): TransformBar {
  const INLINE = 4;
  return { visible: chips.slice(0, INLINE), overflow: chips.slice(INLINE) };
}
