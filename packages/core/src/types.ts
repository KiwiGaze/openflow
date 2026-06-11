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
  /** Brief success flash: ✓ + a preview of the inserted text. */
  | 'inserted'
  | 'notice'
  | 'error';

/** What kind of job the pipeline is currently running. */
export type PipelineJob = 'dictation' | 'refineSelection' | 'polishSelection';

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

/** Window theme. `system` follows macOS; `light`/`dark` force it for OpenFlow. */
export type Appearance = 'system' | 'light' | 'dark';

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
  /**
   * Which `LLM_PRESETS` entry the editor shows — display only. Never changes
   * request behavior (that is `provider` + `baseUrl`); empty for legacy/custom.
   */
  presetId: string;
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
  /**
   * When true the appended "preserve the language" default is dropped so the
   * mode may translate/re-cast (still fenced by the invariant safety rules).
   */
  transforms: boolean;
  /** System prompt used for LLM refinement (the user text only; safety rules
   * are appended at call time). */
  prompt: string;
  // ---- Mode v2 overrides (07); null = inherit the global setting ----
  /** AI profile id, or null to use the active profile. */
  aiProfileId: string | null;
  /** Whisper model id, or null to use the global speech model. */
  sttModelId: string | null;
  /** ISO 639-1 code or `auto`, or null to use the global language. */
  language: string | null;
  /** Accelerator string (e.g. `Alt+Ctrl+N`), or null for no mode hotkey. */
  hotkey: string | null;
}

export interface DictionaryEntry {
  /** What the transcriber tends to produce, e.g. "open flow". */
  from: string;
  /** The replacement, e.g. "OpenFlow". */
  to: string;
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
  /** Whisper model id from the model registry, e.g. `base.en`. */
  sttModelId: string;
  /** ISO 639-1 code or `auto`. */
  language: string;
  insertMethod: InsertMethod;
  restoreClipboard: boolean;
  launchAtLogin: boolean;
  /** Window theme override; `system` defers to macOS. */
  appearance: Appearance;
  /** Opt-in: keep a local, searchable log of past dictations (default off). */
  historyEnabled: boolean;
  /** Master switch for one-time feature tips. */
  tipsEnabled: boolean;
  /** Tip ids already shown; never re-shown. */
  tipsSeen: string[];
  /** Successful dictations ever — the only tip counter (never a log). */
  dictationCount: number;
  /** ISO date (`YYYY-MM-DD`) of the last tip shown; enforces ≤ 1 tip/day. */
  lastTipShownAt: string;
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

/** One entry in the opt-in local dictation history (text only, never audio). */
export interface HistoryEntry {
  id: string;
  raw: string;
  text: string;
  modeId: string;
  refined: boolean;
  /** Unix epoch milliseconds. */
  at: number;
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
  getHistory: 'get_history',
  clearHistory: 'clear_history',
  reprocessHistory: 'reprocess_history',
  testLlm: 'test_llm',
  testMode: 'test_mode',
  listLlmProfiles: 'list_llm_profiles',
  saveLlmProfile: 'save_llm_profile',
  deleteLlmProfile: 'delete_llm_profile',
  revealLlmProfiles: 'reveal_llm_profiles',
  exportMode: 'export_mode',
  exportDictionary: 'export_dictionary',
  listOllamaModels: 'list_ollama_models',
  checkPermissions: 'check_permissions',
  requestMicrophonePermission: 'request_microphone_permission',
  promptAccessibilityPermission: 'prompt_accessibility_permission',
  openAccessibilitySettings: 'open_accessibility_settings',
  openMicrophoneSettings: 'open_microphone_settings',
  getAppInfo: 'get_app_info',
} as const;
