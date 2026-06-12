import type { NoteSummary, Settings, Transform } from '@velata/core';

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

/** A human label for a version's `source` (with the transform name when known). */
export function versionLabel(
  source: string,
  transformId: string | null,
  transforms: Transform[],
): string {
  if (source === 'created') return 'Created';
  if (source === 'restore') return 'Before restore';
  if (source === 'transform') {
    const name = transformId ? transforms.find((t) => t.id === transformId)?.name : undefined;
    return name ? `Before ${name}` : 'Before Polish';
  }
  return source;
}

/** One transform chip: the built-in Polish (null id) plus each settings transform. */
export interface TransformChip {
  /** Null for Polish (server resolves the instruction from polish rules). */
  id: string | null;
  label: string;
}

/** The chip row shown above the editor: Polish first, then transforms by name. */
export function transformChips(settings: Settings): TransformChip[] {
  return [
    { id: null, label: 'Polish' },
    ...settings.transforms.map((t) => ({ id: t.id, label: t.name })),
  ];
}

/** The count line for the main-window tab: "N notes on this Mac". */
export function noteCountLine(notes: NoteSummary[]): string {
  const n = notes.length;
  return `${n} ${n === 1 ? 'note' : 'notes'} on this Mac`;
}
