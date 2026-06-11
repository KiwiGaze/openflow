import { describe, expect, it } from 'vitest';
import {
  isLocalEndpoint,
  isValidBaseUrl,
  normalizeBaseUrl,
  validateDictionaryEntry,
} from './validate.js';

describe('normalizeBaseUrl', () => {
  it('trims whitespace and trailing slashes', () => {
    expect(normalizeBaseUrl(' http://localhost:11434/ ')).toBe('http://localhost:11434');
    expect(normalizeBaseUrl('https://api.openai.com/v1//')).toBe('https://api.openai.com/v1');
  });
});

describe('isValidBaseUrl', () => {
  it('accepts http(s) URLs only', () => {
    expect(isValidBaseUrl('http://localhost:11434')).toBe(true);
    expect(isValidBaseUrl('https://api.groq.com/openai/v1')).toBe(true);
    expect(isValidBaseUrl('ftp://example.com')).toBe(false);
    expect(isValidBaseUrl('localhost:11434')).toBe(false);
    expect(isValidBaseUrl('')).toBe(false);
  });
});

describe('isLocalEndpoint', () => {
  it('treats loopback hosts as local and everything else as cloud', () => {
    expect(isLocalEndpoint('http://localhost:11434')).toBe(true);
    expect(isLocalEndpoint('http://127.0.0.1:1234/v1')).toBe(true);
    expect(isLocalEndpoint('https://api.openai.com/v1')).toBe(false);
    expect(isLocalEndpoint('https://api.groq.com/openai/v1')).toBe(false);
    expect(isLocalEndpoint('not a url')).toBe(false);
  });
});

describe('validateDictionaryEntry', () => {
  const existing = [{ from: 'open flow', to: 'OpenFlow' }];

  it('accepts a valid entry', () => {
    expect(validateDictionaryEntry({ from: 'tory', to: 'Tauri' }, existing)).toBeNull();
  });

  it('rejects empty fields', () => {
    expect(validateDictionaryEntry({ from: ' ', to: 'x' }, existing)).toMatch(/cannot be empty/);
    expect(validateDictionaryEntry({ from: 'x', to: '' }, existing)).toMatch(/cannot be empty/);
  });

  it('rejects duplicates case-insensitively', () => {
    expect(validateDictionaryEntry({ from: 'Open Flow', to: 'y' }, existing)).toMatch(
      /already in the dictionary/,
    );
  });

  it('rejects no-op replacements', () => {
    expect(validateDictionaryEntry({ from: 'same', to: 'Same' }, existing)).toMatch(/identical/);
  });
});
