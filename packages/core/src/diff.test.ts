import { describe, expect, it } from 'vitest';
import { countChanges, diffWords, type DiffRun } from './diff.js';

/** The kept + removed runs must rebuild `before`; kept + added must rebuild `after`. */
function reconstruct(runs: DiffRun[]): { before: string; after: string } {
  let before = '';
  let after = '';
  for (const run of runs) {
    if (run.op !== 'insert') before += run.text;
    if (run.op !== 'delete') after += run.text;
  }
  return { before, after };
}

describe('diffWords', () => {
  it('returns a single equal run for identical text', () => {
    expect(diffWords('hello world', 'hello world')).toEqual([{ op: 'equal', text: 'hello world' }]);
  });

  it('returns nothing for two empty strings', () => {
    expect(diffWords('', '')).toEqual([]);
  });

  it('marks a pure insertion', () => {
    expect(diffWords('', 'new text')).toEqual([{ op: 'insert', text: 'new text' }]);
  });

  it('marks a pure deletion', () => {
    expect(diffWords('old text', '')).toEqual([{ op: 'delete', text: 'old text' }]);
  });

  it('detects a single replaced word and keeps the rest equal', () => {
    const runs = diffWords('the quick fox', 'the slow fox');
    expect(runs).toContainEqual({ op: 'delete', text: 'quick' });
    expect(runs).toContainEqual({ op: 'insert', text: 'slow' });
    expect(reconstruct(runs)).toEqual({ before: 'the quick fox', after: 'the slow fox' });
  });

  it('preserves whitespace so highlights do not swallow spaces', () => {
    const runs = diffWords('a b', 'a c');
    const { before, after } = reconstruct(runs);
    expect(before).toBe('a b');
    expect(after).toBe('a c');
    // The leading "a " is unchanged and stays a single equal run.
    expect(runs[0]).toEqual({ op: 'equal', text: 'a ' });
  });

  it('reconstructs faithfully across multiple edits', () => {
    const before = 'Apologies, I forgot to write it out in full!';
    const after = 'I apologize for not writing it out in full.';
    const runs = diffWords(before, after);
    expect(reconstruct(runs)).toEqual({ before, after });
  });

  it('falls back to a block replace for very long inputs', () => {
    const before = 'word '.repeat(2000).trim();
    const after = 'term '.repeat(2000).trim();
    const runs = diffWords(before, after);
    expect(runs).toEqual([
      { op: 'delete', text: before },
      { op: 'insert', text: after },
    ]);
  });
});

describe('countChanges', () => {
  it('is zero when nothing changed', () => {
    expect(countChanges(diffWords('same', 'same'))).toBe(0);
  });

  it('counts a delete+insert replacement as one change', () => {
    expect(countChanges(diffWords('the quick fox', 'the slow fox'))).toBe(1);
  });

  it('counts separate edits separately', () => {
    expect(countChanges(diffWords('a b c d', 'a x c y'))).toBe(2);
  });
});
