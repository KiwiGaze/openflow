/**
 * TypeScript mirror of the Rust IPC contract.
 *
 * The Rust structs in `apps/desktop/src-tauri/src/settings.rs` and friends use
 * `#[serde(rename_all = "camelCase")]`. Field names and enum string values here
 * must stay in sync with those structs — update both sides together.
 */

export type PipelineStatus =
  | 'idle'
  | 'recording'
  | 'transcribing'
  | 'refining'
  | 'inserting'
  | 'notice'
  | 'error';

/** What kind of job the pipeline is currently running. */
export type PipelineJob = 'dictation' | 'refineSelection' | 'polishSelection' | 'transform';

export interface PipelineState {
  status: PipelineStatus;
  job: PipelineJob | null;
  /** Human-readable detail, set when status is `error`. */
  message: string | null;
}

/** Hold the hotkey to talk, or tap once to start and again to stop. */
export type HotkeyBehavior = 'hold' | 'toggle';

/** `paste` simulates Cmd+V into the active app; `clipboard` only copies. */
export type InsertMethod = 'paste' | 'clipboard';

export type LlmProviderKind = 'ollama' | 'openaiCompatible';

/**
 * One refinement connection, stored as `<app-data>/profiles/<id>.json`.
 * "No AI" is the absence of an active profile, not a provider kind.
 */
export interface LlmProfile {
  /** Schema version of the profile file. */
  version: number;
  /** Identity; always equals the filename stem. */
  id: string;
  name: string;
  provider: LlmProviderKind;
  /** Base URL, e.g. `http://localhost:11434` for Ollama. */
  baseUrl: string;
  /** Bearer token for OpenAI-compatible providers. Empty for Ollama. */
  apiKey: string;
  /** Model name, e.g. `llama3.2:3b` or `gpt-4o-mini`. */
  model: string;
  timeoutSecs: number;
}

export interface Mode {
  id: string;
  name: string;
  builtIn: boolean;
  /**
   * When true the mode wants LLM refinement and falls back to rules-based
   * cleanup if no provider is configured. When false, output is the cleaned
   * transcript only (dictionary still applies).
   */
  usesLlm: boolean;
  /** System prompt used for LLM refinement. */
  prompt: string;
}

export interface DictionaryEntry {
  /** What the transcriber tends to produce, e.g. "open flow". */
  from: string;
  /** The replacement, e.g. "OpenFlow". When equal to `from`, the entry is a
   * pure vocabulary hint (preserve this spelling) rather than a replacement. */
  to: string;
}

/**
 * A term OpenFlow noticed you dictate — a distinctive product/proper name —
 * offered as a one-click dictionary addition. Computed from in-memory,
 * session-only counts; never persisted or transmitted.
 */
export interface DictionarySuggestion {
  term: string;
  count: number;
}

/**
 * A spoken shorthand that expands into a longer block on insert. Unlike a
 * dictionary entry (which fixes a misheard word), a snippet is intentional
 * abbreviation: short trigger → long, possibly multi-line, verbatim text.
 */
export interface Snippet {
  /** The spoken phrase that triggers expansion, e.g. "my email". */
  trigger: string;
  /** Text inserted in place of the trigger; may span multiple lines. */
  expansion: string;
  /**
   * When true, expand only if the trigger is the whole dictation — for
   * triggers that also occur in ordinary prose ("my email").
   */
  wholeUtterance: boolean;
}

/**
 * A named, one-tap text operation applied to the current selection — a saved
 * Rewrite instruction with its own hotkey. Polish is the built-in default of
 * the same shape.
 */
export interface Transform {
  /** Stable identity (a UUID); the hotkey resolves the instruction by this. */
  id: string;
  name: string;
  /** Instruction sent to the active profile alongside the selection. */
  instruction: string;
  /** Accelerator that applies it; empty = not yet bound (can't fire). */
  hotkey: string;
}

export interface Settings {
  /** Schema version for forward migrations. */
  version: number;
  dictationHotkey: string;
  dictationHotkeyBehavior: HotkeyBehavior;
  refineHotkey: string;
  polishHotkey: string;
  /** Master switch: may dictation transcripts go to the active profile. */
  refineAfterDictation: boolean;
  /** Active profile id (a file under `<app-data>/profiles/`); "" = no AI. */
  activeLlmProfileId: string;
  activeModeId: string;
  modes: Mode[];
  dictionary: DictionaryEntry[];
  /** Spoken shorthands expanded into longer blocks on insert (dictation only). */
  snippets: Snippet[];
  /** Named, hotkey-bound text operations applied to a selection. */
  transforms: Transform[];
  /** Whisper model id from the model registry, e.g. `base.en`. */
  sttModelId: string;
  /** ISO 639-1 code or `auto`. */
  language: string;
  insertMethod: InsertMethod;
  restoreClipboard: boolean;
  launchAtLogin: boolean;
  onboardingCompleted: boolean;
}

export interface ModelInfo {
  id: string;
  displayName: string;
  fileName: string;
  url: string;
  sizeBytes: number;
  multilingual: boolean;
  /** Short guidance shown in the picker, e.g. "fastest" / "best quality". */
  description: string;
  installed: boolean;
  downloading: boolean;
}

export interface DownloadProgress {
  modelId: string;
  downloadedBytes: number;
  totalBytes: number;
  done: boolean;
  error: string | null;
}

export interface TranscriptionResult {
  /** Raw whisper output after basic trimming. */
  raw: string;
  /** Final text that was inserted. */
  text: string;
  modeId: string;
  /** Whether an LLM pass ran (false means rules-based cleanup only). */
  refined: boolean;
  durationMs: number;
}

export interface ModeCount {
  modeId: string;
  count: number;
}

/**
 * Session-only usage aggregates for the Insights view. Counts and sums only —
 * never transcripts or audio — held in memory and reset on quit. Mirrors the
 * Rust `Insights` in `stats.rs`.
 */
export interface Insights {
  totalWords: number;
  dictations: number;
  /** Average speaking pace this session; 0 until some speech is recorded. */
  wordsPerMinute: number;
  /** Percent of dictations that went through the LLM (vs rules cleanup). */
  refinedPercent: number;
  /** Most-used modes, highest first (up to 3). */
  topModes: ModeCount[];
}

export type MicrophonePermission = 'granted' | 'denied' | 'undetermined' | 'unknown';

export interface PermissionsState {
  microphone: MicrophonePermission;
  accessibility: boolean;
}

export interface LlmTestResult {
  ok: boolean;
  message: string;
}

export interface AppInfo {
  version: string;
  dataDir: string;
  configPath: string;
}

/** Tauri event names emitted by the Rust backend. */
export const EVENTS = {
  pipelineState: 'pipeline-state',
  audioLevel: 'audio-level',
  modelDownload: 'model-download',
  settingsChanged: 'settings-changed',
  result: 'transcription-result',
} as const;

/** Tauri command names callable via `invoke`. */
export const COMMANDS = {
  getSettings: 'get_settings',
  saveSettings: 'save_settings',
  listModels: 'list_models',
  downloadModel: 'download_model',
  cancelModelDownload: 'cancel_model_download',
  deleteModel: 'delete_model',
  getPipelineState: 'get_pipeline_state',
  startDictation: 'start_dictation',
  stopDictation: 'stop_dictation',
  cancelDictation: 'cancel_dictation',
  startRefineSelection: 'start_refine_selection',
  startPolishSelection: 'start_polish_selection',
  getLastResult: 'get_last_result',
  getInsights: 'get_insights',
  listDictionarySuggestions: 'list_dictionary_suggestions',
  dismissDictionarySuggestion: 'dismiss_dictionary_suggestion',
  testLlm: 'test_llm',
  listLlmProfiles: 'list_llm_profiles',
  saveLlmProfile: 'save_llm_profile',
  deleteLlmProfile: 'delete_llm_profile',
  revealLlmProfiles: 'reveal_llm_profiles',
  listOllamaModels: 'list_ollama_models',
  checkPermissions: 'check_permissions',
  requestMicrophonePermission: 'request_microphone_permission',
  promptAccessibilityPermission: 'prompt_accessibility_permission',
  openAccessibilitySettings: 'open_accessibility_settings',
  openMicrophoneSettings: 'open_microphone_settings',
  getAppInfo: 'get_app_info',
} as const;
