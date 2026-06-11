import { describe, expect, it } from 'vitest';
import { LANGUAGES, languageLabel } from './languages.js';

describe('languageLabel', () => {
  it('returns the label for a known code', () => {
    expect(languageLabel('en')).toBe('English');
    expect(languageLabel('auto')).toBe('Auto-detect');
  });

  it('falls back to the raw code for unknown codes', () => {
    expect(languageLabel('xx')).toBe('xx');
    expect(languageLabel('')).toBe('');
  });

  it('keeps auto-detect as the first option', () => {
    expect(LANGUAGES[0]).toEqual(['auto', 'Auto-detect']);
  });
});
