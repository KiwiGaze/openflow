import { describe, expect, it } from 'vitest';
import {
  MODE_SCHEMA,
  parseModeImport,
  serializeMode,
  slugifyMode,
  uniqueModeName,
} from './modeio.js';
import type { Mode } from './types.js';

const mode: Mode = {
  id: 'abc',
  name: 'Standup update',
  builtIn: false,
  usesLlm: true,
  transforms: false,
  prompt: 'Turn dictation into a short standup update.',
  aiProfileId: 'p1',
  sttModelId: 'base.en',
  language: 'en',
  hotkey: 'Alt+Ctrl+N',
};

describe('serializeMode', () => {
  it('exports only portable content — never id, builtIn, or hotkey', () => {
    const json = JSON.parse(serializeMode(mode, '2026-06-11')) as Record<string, unknown>;
    expect(json.schema).toBe(MODE_SCHEMA);
    expect(json.mode).toEqual({
      name: 'Standup update',
      usesLlm: true,
      transforms: false,
      language: 'en',
      prompt: 'Turn dictation into a short standup update.',
    });
  });

  it('round-trips through parseModeImport', () => {
    const result = parseModeImport(serializeMode(mode, '2026-06-11'));
    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.mode.name).toBe('Standup update');
      expect(result.mode.builtIn).toBe(false);
      expect(result.mode.hotkey).toBeNull(); // never imported
      expect(result.mode.aiProfileId).toBeNull();
    }
  });
});

describe('parseModeImport', () => {
  it('rejects non-JSON, wrong schema, and a newer major version', () => {
    expect(parseModeImport('{ not json').ok).toBe(false);
    expect(parseModeImport('{"schema":"something/1","mode":{}}').ok).toBe(false);
    const newer = parseModeImport('{"schema":"velata.mode/2","mode":{"name":"x","prompt":"y"}}');
    expect(newer).toEqual({
      ok: false,
      error: 'This mode was made with a newer version of Velata.',
    });
  });

  it('rejects an empty name or prompt', () => {
    expect(parseModeImport('{"schema":"velata.mode/1","mode":{"name":" ","prompt":"y"}}').ok).toBe(
      false,
    );
  });

  it('defaults missing flags, clamps an invalid language to null', () => {
    const r = parseModeImport(
      '{"schema":"velata.mode/1","mode":{"name":"X","prompt":"do it","language":"english"}}',
    );
    expect(r.ok).toBe(true);
    if (r.ok) {
      expect(r.mode.usesLlm).toBe(false);
      expect(r.mode.transforms).toBe(false);
      expect(r.mode.language).toBeNull();
    }
  });

  it('round-trips the auto-detect language', () => {
    const r = parseModeImport(
      '{"schema":"velata.mode/1","mode":{"name":"X","prompt":"do it","language":"auto"}}',
    );
    expect(r.ok).toBe(true);
    if (r.ok) {
      expect(r.mode.language).toBe('auto');
    }
  });
});

describe('slugifyMode / uniqueModeName', () => {
  it('slugifies names and falls back to "mode"', () => {
    expect(slugifyMode('Standup Update!')).toBe('standup-update');
    expect(slugifyMode('   ')).toBe('mode');
  });

  it('appends a counter on a name collision', () => {
    expect(uniqueModeName('Email', ['Email', 'Email (2)'])).toBe('Email (3)');
    expect(uniqueModeName('Notes', ['Email'])).toBe('Notes');
  });
});
