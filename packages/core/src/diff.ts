/**
 * Word-level diff for the "see changes" overlay.
 *
 * Splits both strings into word and whitespace tokens, runs a longest-common-
 * subsequence pass, and emits runs the UI can render as kept / removed / added
 * text. Whitespace is tokenized too so the reconstruction is faithful and a
 * changed word does not drag its surrounding spaces into the highlight.
 *
 * This is post-hoc UI, never on the dictation critical path, so a quadratic LCS
 * is fine for the short texts dictation and polish produce. Pathologically long
 * inputs fall back to a single remove/add block to bound memory.
 */

export type DiffOp = 'equal' | 'insert' | 'delete';

export interface DiffRun {
  op: DiffOp;
  text: string;
}

/** Above this token count per side, skip the LCS table and block-replace. */
const MAX_TOKENS = 1500;

function tokenize(input: string): string[] {
  return input.match(/\s+|\S+/g) ?? [];
}

/** Coalesces adjacent runs that share an op so the DOM stays small. */
function mergeRuns(runs: DiffRun[]): DiffRun[] {
  const merged: DiffRun[] = [];
  for (const run of runs) {
    if (run.text.length === 0) continue;
    const last = merged[merged.length - 1];
    if (last?.op === run.op) {
      last.text += run.text;
    } else {
      merged.push({ op: run.op, text: run.text });
    }
  }
  return merged;
}

export function diffWords(before: string, after: string): DiffRun[] {
  if (before === after) {
    return before.length > 0 ? [{ op: 'equal', text: before }] : [];
  }

  const a = tokenize(before);
  const b = tokenize(after);

  if (a.length > MAX_TOKENS || b.length > MAX_TOKENS) {
    return mergeRuns([
      { op: 'delete', text: before },
      { op: 'insert', text: after },
    ]);
  }

  const n = a.length;
  const m = b.length;
  // lcs[i][j] = LCS length of a[i:] and b[j:], stored flat as (n+1)*(m+1).
  const width = m + 1;
  const lcs = new Int32Array((n + 1) * width);
  for (let i = n - 1; i >= 0; i--) {
    for (let j = m - 1; j >= 0; j--) {
      if (a[i] === b[j]) {
        lcs[i * width + j] = (lcs[(i + 1) * width + (j + 1)] ?? 0) + 1;
      } else {
        lcs[i * width + j] = Math.max(lcs[(i + 1) * width + j] ?? 0, lcs[i * width + (j + 1)] ?? 0);
      }
    }
  }

  const runs: DiffRun[] = [];
  let i = 0;
  let j = 0;
  while (i < n && j < m) {
    if (a[i] === b[j]) {
      runs.push({ op: 'equal', text: a[i] ?? '' });
      i++;
      j++;
    } else if ((lcs[(i + 1) * width + j] ?? 0) >= (lcs[i * width + (j + 1)] ?? 0)) {
      runs.push({ op: 'delete', text: a[i] ?? '' });
      i++;
    } else {
      runs.push({ op: 'insert', text: b[j] ?? '' });
      j++;
    }
  }
  while (i < n) {
    runs.push({ op: 'delete', text: a[i] ?? '' });
    i++;
  }
  while (j < m) {
    runs.push({ op: 'insert', text: b[j] ?? '' });
    j++;
  }

  return mergeRuns(runs);
}

/**
 * Counts edits as a human would: each maximal stretch of removed and/or added
 * runs between kept text is one change, so a replaced word (delete + insert)
 * counts once, not twice.
 */
export function countChanges(runs: DiffRun[]): number {
  let count = 0;
  let inChange = false;
  for (const run of runs) {
    if (run.op === 'equal') {
      inChange = false;
    } else if (!inChange) {
      count++;
      inChange = true;
    }
  }
  return count;
}
