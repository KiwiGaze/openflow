/** Spoken-language options for the STT language hint: `[code, label]`. */
export const LANGUAGES: readonly (readonly [string, string])[] = [
  ['auto', 'Auto-detect'],
  ['en', 'English'],
  ['zh', 'Chinese'],
  ['es', 'Spanish'],
  ['fr', 'French'],
  ['de', 'German'],
  ['ja', 'Japanese'],
  ['ko', 'Korean'],
  ['pt', 'Portuguese'],
  ['ru', 'Russian'],
  ['it', 'Italian'],
  ['nl', 'Dutch'],
  ['hi', 'Hindi'],
  ['ar', 'Arabic'],
];

/** Human label for a language code, falling back to the raw code. */
export function languageLabel(code: string): string {
  return LANGUAGES.find(([c]) => c === code)?.[1] ?? code;
}
