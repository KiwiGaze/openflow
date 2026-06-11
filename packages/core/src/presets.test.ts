import { describe, expect, it } from 'vitest';
import { LLM_PRESETS, presetForProfile } from './presets.js';

describe('presetForProfile', () => {
  it('returns the preset matching an explicit id', () => {
    expect(presetForProfile('groq', 'openaiCompatible').displayName).toBe('Groq');
  });

  it('falls back by wire kind for legacy profiles with no presetId', () => {
    expect(presetForProfile('', 'ollama').id).toBe('ollama');
    expect(presetForProfile('', 'openaiCompatible').id).toBe('custom');
  });
});

describe('LLM_PRESETS', () => {
  it('uses only the two real wire kinds — presets are data, not new code paths', () => {
    for (const preset of LLM_PRESETS) {
      expect(['ollama', 'openaiCompatible']).toContain(preset.kind);
    }
  });

  it('marks every cloud-key preset as needing a key', () => {
    const openai = LLM_PRESETS.find((p) => p.id === 'openai');
    expect(openai?.needsKey).toBe(true);
    const lmstudio = LLM_PRESETS.find((p) => p.id === 'lmstudio');
    expect(lmstudio?.needsKey).toBe(false);
  });
});
