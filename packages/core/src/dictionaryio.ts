import type { DictionaryEntry } from './types.js';
import { validateDictionaryEntry } from './validate.js';

function csvField(value: string): string {
  if (value.includes(',') || value.includes('"') || value.includes('\n') || value.includes('\r')) {
    return `"${value.replace(/"/g, '""')}"`;
  }
  return value;
}

/** Serialize dictionary entries to CSV with a `from,to` header. */
export function dictionaryToCsv(entries: readonly DictionaryEntry[]): string {
  const rows = ['from,to', ...entries.map((e) => `${csvField(e.from)},${csvField(e.to)}`)];
  return `${rows.join('\n')}\n`;
}

/** Minimal RFC-4180 parser: quoted fields, "" escapes, LF or CRLF rows. */
function parseCsv(text: string): string[][] {
  const rows: string[][] = [];
  let field = '';
  let row: string[] = [];
  let inQuotes = false;
  for (let i = 0; i < text.length; i += 1) {
    const ch = text.charAt(i);
    if (inQuotes) {
      if (ch === '"' && text.charAt(i + 1) === '"') {
        field += '"';
        i += 1;
      } else if (ch === '"') {
        inQuotes = false;
      } else {
        field += ch;
      }
    } else if (ch === '"') {
      inQuotes = true;
    } else if (ch === ',') {
      row.push(field);
      field = '';
    } else if (ch === '\n' || ch === '\r') {
      if (ch === '\r' && text.charAt(i + 1) === '\n') i += 1;
      row.push(field);
      field = '';
      rows.push(row);
      row = [];
    } else {
      field += ch;
    }
  }
  if (field !== '' || row.length > 0) {
    row.push(field);
    rows.push(row);
  }
  return rows;
}

export interface DictionaryCsvResult {
  entries: DictionaryEntry[];
  skipped: number;
}

/**
 * Parse a CSV file into dictionary entries, dropping the optional `from,to`
 * header and any row that fails the same validation as the add form (empty,
 * too long, identical, or a duplicate of an existing or earlier-parsed entry).
 */
export function parseDictionaryCsv(
  text: string,
  existing: readonly DictionaryEntry[],
): DictionaryCsvResult {
  const rows = parseCsv(text);
  const entries: DictionaryEntry[] = [];
  const accumulated = [...existing];
  let skipped = 0;
  for (const [index, row] of rows.entries()) {
    const from = (row[0] ?? '').trim();
    const to = (row[1] ?? '').trim();
    if (index === 0 && from.toLowerCase() === 'from' && to.toLowerCase() === 'to') continue;
    if (from === '' && to === '') continue; // blank line
    const entry = { from, to };
    if (validateDictionaryEntry(entry, accumulated) !== null) {
      skipped += 1;
      continue;
    }
    entries.push(entry);
    accumulated.push(entry);
  }
  return { entries, skipped };
}
