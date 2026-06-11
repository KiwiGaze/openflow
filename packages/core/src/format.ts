/** Format a byte count for humans, e.g. 148897792 → `148.9 MB`. */
export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes < 0) return '—';
  if (bytes < 1000) return `${bytes} B`;
  const units = ['kB', 'MB', 'GB', 'TB'];
  let value = bytes;
  let unit = 'B';
  for (const next of units) {
    if (value < 1000) break;
    value /= 1000;
    unit = next;
  }
  return `${value.toFixed(value >= 100 ? 0 : 1)} ${unit}`;
}

/** Format milliseconds as a short duration, e.g. 1234 → `1.2s`. */
export function formatDurationMs(ms: number): string {
  if (!Number.isFinite(ms) || ms < 0) return '—';
  if (ms < 1000) return `${Math.round(ms)}ms`;
  const secs = ms / 1000;
  if (secs < 60) return `${secs.toFixed(1)}s`;
  const mins = Math.floor(secs / 60);
  const rest = Math.round(secs % 60);
  return `${mins}m ${rest}s`;
}

/** Download progress as a percentage string; `—` until the total is known. */
export function formatProgress(downloaded: number, total: number): string {
  if (total <= 0) return '—';
  const pct = Math.min(100, (downloaded / total) * 100);
  return `${pct.toFixed(0)}%`;
}

/** Today as a `YYYY-MM-DD` string (UTC calendar date) — tip caps, export stamps. */
export function todayIso(): string {
  return new Date().toISOString().slice(0, 10);
}
