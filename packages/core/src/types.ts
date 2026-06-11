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
  /** Reveals the word-level diff of the last result. Empty disables it. */
  changeOverlayHotkey: string;
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
  /**
   * The text the change is measured against for the "see changes" diff:
   * the transcript for dictation, the original selection for polish/rewrite.
   * Differs from `raw` only for rewrite, where `raw` is the spoken instruction.
   */
  original: string;
  /** Final text that was inserted. */
  text: string;
  modeId: string;
  /** Whether an LLM pass ran (false means rules-based cleanup only). */
  refined: boolean;
  durationMs: number;
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
  changesToggle: 'changes-toggle',
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
  copyText: 'copy_text',
  setChangesInteractive: 'set_changes_interactive',
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
