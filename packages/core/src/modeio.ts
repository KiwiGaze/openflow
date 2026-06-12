import type { Mode } from './types.js';

export const MODE_SCHEMA = 'velata.mode/1';
const SCHEMA_PREFIX = 'velata.mode/';
const MAX_NAME = 80;
const MAX_PROMPT = 8000;
const MAX_FILE_BYTES = 64 * 1024;

export interface ModeExport {
  schema: string;
  /** Local date, informational only. */
  exportedAt: string;
  mode: {
    name: string;
    usesLlm: boolean;
    transforms: boolean;
    language: string | null;
    prompt: string;
  };
}

/**
 * The portable export envelope (06 §4): only shareable content travels. `id`,
 * `builtIn`, and `hotkey` are never exported — identity and key bindings are
 * local.
 */
export function serializeMode(mode: Mode, today: string): string {
  const envelope: ModeExport = {
    schema: MODE_SCHEMA,
    exportedAt: today,
    mode: {
      name: mode.name,
      usesLlm: mode.usesLlm,
      transforms: mode.transforms,
      language: mode.language,
      prompt: mode.prompt,
    },
  };
  return JSON.stringify(envelope, null, 2);
}

/** Filename slug for an exported mode, e.g. "Standup update" → "standup-update". */
export function slugifyMode(name: string): string {
  const slug = name
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '');
  return slug === '' ? 'mode' : slug;
}

/** Append " (2)", "(3)", … until the name is unique, like the Finder (06 §4). */
export function uniqueModeName(name: string, existingNames: readonly string[]): string {
  if (!existingNames.includes(name)) return name;
  for (let i = 2; ; i += 1) {
    const candidate = `${name} (${i})`;
    if (!existingNames.includes(candidate)) return candidate;
  }
}

/** Drop control characters except tab and newline (the import trust boundary). */
function stripControl(text: string): string {
  return Array.from(text)
    .filter((ch) => {
      const code = ch.codePointAt(0) ?? 0;
      return code === 9 || code === 10 || code >= 32;
    })
    .join('');
}

export type ModeImportResult = { ok: true; mode: Omit<Mode, 'id'> } | { ok: false; error: string };

/**
 * Validate the §4 import JSON. Returns the mode fields (the caller assigns a
 * fresh id and resolves name collisions). An imported prompt is still just a
 * mode prompt, fenced by SAFETY_RULES at runtime.
 */
export function parseModeImport(text: string): ModeImportResult {
  if (text.length > MAX_FILE_BYTES) {
    return { ok: false, error: 'That file is too large to be a mode.' };
  }
  let data: unknown;
  try {
    data = JSON.parse(text);
  } catch {
    return { ok: false, error: "This file isn't a valid mode." };
  }
  if (typeof data !== 'object' || data === null) {
    return { ok: false, error: "This file isn't a valid mode." };
  }
  const envelope = data as Record<string, unknown>;
  const schema = envelope.schema;
  if (typeof schema !== 'string' || !schema.startsWith(SCHEMA_PREFIX)) {
    return { ok: false, error: "This file isn't a valid mode." };
  }
  if (schema !== MODE_SCHEMA) {
    return { ok: false, error: 'This mode was made with a newer version of Velata.' };
  }
  const raw = envelope.mode;
  if (typeof raw !== 'object' || raw === null) {
    return { ok: false, error: "This file isn't a valid mode." };
  }
  const mode = raw as Record<string, unknown>;
  const name = typeof mode.name === 'string' ? mode.name.trim() : '';
  const prompt = typeof mode.prompt === 'string' ? mode.prompt : '';
  if (name === '' || prompt.trim() === '') {
    return { ok: false, error: "This file isn't a valid mode." };
  }
  // `auto` (auto-detect) is a valid stored language alongside ISO 639-1 codes.
  const language =
    typeof mode.language === 'string' && /^([a-z]{2}|auto)$/.test(mode.language)
      ? mode.language
      : null;
  return {
    ok: true,
    mode: {
      name: stripControl(name).slice(0, MAX_NAME),
      builtIn: false,
      usesLlm: typeof mode.usesLlm === 'boolean' ? mode.usesLlm : false,
      transforms: typeof mode.transforms === 'boolean' ? mode.transforms : false,
      prompt: stripControl(prompt).slice(0, MAX_PROMPT),
      aiProfileId: null,
      sttModelId: null,
      language,
      hotkey: null,
    },
  };
}
