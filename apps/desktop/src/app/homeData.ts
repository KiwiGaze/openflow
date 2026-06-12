/** Pure presentation logic for the Home tab: greeting + day grouping. */

import type { HistoryEntry } from '@velata/core';

/** A day's worth of history entries, labeled for display. */
export interface HistoryDayGroup {
  /** Stable key: the local calendar day as `YYYY-MM-DD`. */
  key: string;
  /** Human label: "Today", "Yesterday", or a readable date. */
  label: string;
  /** Entries for this day, in the order received (the API returns newest first). */
  entries: HistoryEntry[];
}

/** Time-of-day greeting from a local 24h hour (0–23). */
export function greetingForHour(hour: number): string {
  if (hour < 12) return 'Good morning';
  if (hour < 18) return 'Good afternoon';
  return 'Good evening';
}

/** Local calendar day as `YYYY-MM-DD`, so two timestamps on the same day match. */
function dayKey(date: Date): string {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, '0');
  const day = String(date.getDate()).padStart(2, '0');
  return `${year}-${month}-${day}`;
}

function dayLabel(entryDay: Date, now: Date): string {
  const todayKey = dayKey(now);
  const yesterday = new Date(now);
  yesterday.setDate(yesterday.getDate() - 1);
  const key = dayKey(entryDay);
  if (key === todayKey) return 'Today';
  if (key === dayKey(yesterday)) return 'Yesterday';
  // Year shown only when it differs from the current year, so "June 10" stays
  // terse within this year and disambiguates older entries.
  const withYear = entryDay.getFullYear() !== now.getFullYear();
  return new Intl.DateTimeFormat(undefined, {
    month: 'long',
    day: 'numeric',
    year: withYear ? 'numeric' : undefined,
  }).format(entryDay);
}

/**
 * Groups history entries by local calendar day, newest day first, preserving
 * each group's incoming order (the API already returns newest first).
 */
export function groupHistoryByDay(entries: HistoryEntry[], now: Date): HistoryDayGroup[] {
  const groups: HistoryDayGroup[] = [];
  const byKey = new Map<string, HistoryDayGroup>();
  for (const entry of entries) {
    const entryDay = new Date(entry.at);
    const key = dayKey(entryDay);
    let group = byKey.get(key);
    if (!group) {
      group = { key, label: dayLabel(entryDay, now), entries: [] };
      byKey.set(key, group);
      groups.push(group);
    }
    group.entries.push(entry);
  }
  return groups;
}
