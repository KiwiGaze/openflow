import { describe, expect, it } from 'vitest';
import { dictionaryToCsv, parseDictionaryCsv } from './dictionaryio.js';

describe('dictionaryToCsv', () => {
  it('writes a header and quotes fields with commas or quotes', () => {
    const csv = dictionaryToCsv([
      { from: 'open flow', to: 'OpenFlow' },
      { from: 'a, b', to: 'c"d' },
    ]);
    expect(csv).toBe('from,to\nopen flow,OpenFlow\n"a, b","c""d"\n');
  });
});

describe('parseDictionaryCsv', () => {
  it('round-trips through dictionaryToCsv', () => {
    const entries = [
      { from: 'open flow', to: 'OpenFlow' },
      { from: 'tory', to: 'Tauri' },
    ];
    const result = parseDictionaryCsv(dictionaryToCsv(entries), []);
    expect(result.entries).toEqual(entries);
    expect(result.skipped).toBe(0);
  });

  it('skips the header, blank lines, and invalid or duplicate rows', () => {
    const text = 'from,to\nopen flow,OpenFlow\n\nbad,\nopen flow,OpenFlow\n';
    const result = parseDictionaryCsv(text, []);
    expect(result.entries).toEqual([{ from: 'open flow', to: 'OpenFlow' }]);
    expect(result.skipped).toBe(2); // "bad," (empty `to`) and the duplicate
  });

  it('treats entries already present as duplicates', () => {
    const result = parseDictionaryCsv('tory,Tauri\n', [{ from: 'tory', to: 'Tauri' }]);
    expect(result.entries).toEqual([]);
    expect(result.skipped).toBe(1);
  });
});
