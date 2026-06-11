import type { LlmProviderKind } from './types.js';

/** A provider prefill for the cloud/remote STT engine editor (08 §2). */
export interface SttPreset {
  /** Stored on the profile as `presetId`; display-only, never changes behavior. */
  id: string;
  label: string;
  /** Prefilled base URL; the user may edit it freely. Empty for `custom`. */
  baseUrl: string;
  /** Suggested model — a prefill, never locked. Empty = user fills. */
  model: string;
}

/** What the STT quick-add button creates before the user edits anything. */
export const DEFAULT_STT_PRESET: SttPreset = {
  id: 'groqStt',
  label: 'Groq',
  baseUrl: 'https://api.groq.com/openai/v1',
  model: 'whisper-large-v3',
};

/**
 * Provider prefills over the one OpenAI-audio multipart client. Like
 * `LLM_PRESETS`, a preset is display + prefill, not a code path.
 */
export const STT_PRESETS: SttPreset[] = [
  DEFAULT_STT_PRESET,
  { id: 'openaiStt', label: 'OpenAI', baseUrl: 'https://api.openai.com/v1', model: 'whisper-1' },
  {
    id: 'whisperServer',
    label: 'Local whisper-server',
    baseUrl: 'http://localhost:8080/v1',
    model: 'whisper-1',
  },
  { id: 'custom', label: 'Custom (OpenAI-audio)', baseUrl: '', model: '' },
];

export interface LlmPreset {
  /** Stored on the profile as `presetId`; display-only, never changes behavior. */
  id: string;
  displayName: string;
  /** Prefilled base URL; the user may edit it freely. Empty for `custom`. */
  baseUrl: string;
  /** Which wire kind the single client uses. */
  kind: LlmProviderKind;
  /** Whether the editor shows the API-key field. */
  needsKey: boolean;
  /** Suggested model — a placeholder/prefill, never locked. Empty = user fills. */
  modelSuggestion: string;
  /** One-line caveat shown under the preset. */
  caveat: string;
}

/** The escape hatch: raw fields, any OpenAI-compatible endpoint. */
const CUSTOM_PRESET: LlmPreset = {
  id: 'custom',
  displayName: 'Custom (OpenAI-compatible)',
  baseUrl: '',
  kind: 'openaiCompatible',
  needsKey: true,
  modelSuggestion: '',
  caveat: 'Any OpenAI-compatible endpoint — fill the fields yourself.',
};

/**
 * Provider prefills over the one OpenAI-compatible client (08 §1). Every cloud
 * provider here speaks `/v1/chat/completions`, so a preset is display + prefill,
 * not a code path — `LlmProviderKind` stays `ollama | openaiCompatible`.
 * Locality (local/cloud) is derived from the base-URL host, never from the
 * preset. Model ids drift; these are suggestions the user can change.
 */
/** The default local provider; the quick-add path constructs profiles from it. */
export const OLLAMA_PRESET: LlmPreset = {
  id: 'ollama',
  displayName: 'Ollama',
  baseUrl: 'http://localhost:11434',
  kind: 'ollama',
  needsKey: false,
  modelSuggestion: 'qwen2.5:3b',
  caveat: 'Local. Use “List installed models” to pick one.',
};

export const LLM_PRESETS: LlmPreset[] = [
  OLLAMA_PRESET,
  {
    id: 'lmstudio',
    displayName: 'LM Studio',
    baseUrl: 'http://localhost:1234/v1',
    kind: 'openaiCompatible',
    needsKey: false,
    modelSuggestion: '',
    caveat: 'Local server — start it and load a model first.',
  },
  {
    id: 'llamacpp',
    displayName: 'llama.cpp server',
    baseUrl: 'http://localhost:8080/v1',
    kind: 'openaiCompatible',
    needsKey: false,
    modelSuggestion: '',
    caveat: 'llama-server serves one model; the model field may be ignored.',
  },
  {
    id: 'openai',
    displayName: 'OpenAI',
    baseUrl: 'https://api.openai.com/v1',
    kind: 'openaiCompatible',
    needsKey: true,
    modelSuggestion: 'gpt-4o-mini',
    caveat: 'Cloud — refined text (never audio) leaves your Mac.',
  },
  {
    id: 'groq',
    displayName: 'Groq',
    baseUrl: 'https://api.groq.com/openai/v1',
    kind: 'openaiCompatible',
    needsKey: true,
    modelSuggestion: 'llama-3.3-70b-versatile',
    caveat: 'Cloud — OpenAI-compatible and very fast.',
  },
  {
    id: 'openrouter',
    displayName: 'OpenRouter',
    baseUrl: 'https://openrouter.ai/api/v1',
    kind: 'openaiCompatible',
    needsKey: true,
    modelSuggestion: 'openai/gpt-4o-mini',
    caveat: 'Cloud — model ids are namespaced vendor/model.',
  },
  {
    id: 'mistral',
    displayName: 'Mistral',
    baseUrl: 'https://api.mistral.ai/v1',
    kind: 'openaiCompatible',
    needsKey: true,
    modelSuggestion: 'mistral-small-latest',
    caveat: 'Cloud — OpenAI-compatible chat endpoint.',
  },
  {
    id: 'anthropic',
    displayName: 'Anthropic (Claude)',
    baseUrl: 'https://api.anthropic.com/v1',
    kind: 'openaiCompatible',
    needsKey: true,
    modelSuggestion: 'claude-sonnet-4-6',
    caveat: 'Cloud — via Anthropic’s OpenAI-compatible layer (text only).',
  },
  {
    id: 'gemini',
    displayName: 'Google Gemini',
    baseUrl: 'https://generativelanguage.googleapis.com/v1beta/openai',
    kind: 'openaiCompatible',
    needsKey: true,
    modelSuggestion: 'gemini-2.0-flash',
    caveat: 'Cloud — uses the OpenAI-compatible chat path.',
  },
  CUSTOM_PRESET,
];

/**
 * The preset a profile's editor should show. Falls back to the wire kind for
 * legacy profiles with no `presetId` (ollama → Ollama, else → Custom).
 */
export function presetForProfile(presetId: string, provider: LlmProviderKind): LlmPreset {
  const byId = LLM_PRESETS.find((p) => p.id === presetId);
  if (byId) return byId;
  const fallbackId = provider === 'ollama' ? 'ollama' : 'custom';
  return LLM_PRESETS.find((p) => p.id === fallbackId) ?? CUSTOM_PRESET;
}
