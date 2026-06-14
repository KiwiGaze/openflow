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
  | 'polishing'
  | 'inserting'
  /** Brief success flash: ✓ + a preview of the inserted text. */
  | 'inserted'
  | 'notice'
  | 'error';

/** What kind of job the pipeline is currently running. */
export type PipelineJob = 'dictation' | 'transform';

export interface PipelineState {
  status: PipelineStatus;
  job: PipelineJob | null;
  /** Human-readable detail, set when status is `error`. */
  message: string | null;
  /** One-time educational tip shown on the success flash; null otherwise. */
  hudTip: string | null;
}

/**
 * How a hotkey is triggered. `hold`/`doubleTap` describe an `fn`-key gesture
 * (observation lands in Phase 3; until then the Rust side falls them back to an
 * accelerator); `accelerator` is a literal combo like `Alt+O`.
 */
export type HotkeyKind = 'hold' | 'doubleTap' | 'accelerator';

/**
 * A gesture trigger: a `kind` plus its `key`. `key` is `'fn'` for the gesture
 * defaults, or an accelerator string (e.g. `'Alt+O'`) when `kind` is
 * `'accelerator'`. Mirrors `Hotkey` in `settings.rs`.
 */
export interface Hotkey {
  kind: HotkeyKind;
  key: string;
}

/** Window theme. `system` follows macOS; `light`/`dark` force it for Velata. */
export type Appearance = 'system' | 'light' | 'dark';

export type LlmProviderKind = 'ollama' | 'openaiCompatible';

/** Schema version written to new LLM profile files. Mirrors `PROFILE_VERSION` in `profiles.rs`. */
export const LLM_PROFILE_VERSION = 1;

/**
 * One AI-polish connection, stored as `<app-data>/profiles/<id>.json`.
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

/** Which STT client transcribes. Only `openaiAudio` ships now (08 §2). */
export type SttEngineKind = 'openaiAudio';

/** Schema version written to new STT profile files. Mirrors `STT_PROFILE_VERSION` in `stt_profiles.rs`. */
export const STT_PROFILE_VERSION = 1;

/**
 * `Settings.sttModelId` prefix marking a cloud engine: `cloud:<profile-id>`.
 * Anything else is an on-device whisper model id. Mirrors `CLOUD_STT_PREFIX`
 * in `stt_profiles.rs` — update both together.
 */
export const CLOUD_STT_PREFIX = 'cloud:';

/**
 * One cloud/remote STT connection, stored as `<app-data>/stt-profiles/<id>.json`.
 * The on-device whisper default needs no profile.
 */
export interface SttProfile {
  version: number;
  id: string;
  name: string;
  engine: SttEngineKind;
  /** Display/prefill only; never changes request behavior. */
  presetId: string;
  baseUrl: string;
  apiKey: string;
  model: string;
  timeoutSecs: number;
}

export interface DictionaryEntry {
  /** What the transcriber tends to produce, e.g. "open flow". */
  from: string;
  /** The replacement, e.g. "Velata". When equal to `from`, the entry is a
   * pure vocabulary hint (preserve this spelling) rather than a replacement. */
  to: string;
}

/**
 * A term Velata noticed you dictate — a distinctive product/proper name —
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
 * A named, one-tap text operation applied to a selection — a saved instruction
 * with its own shortcut (Transforms page). The built-in Polish is the same
 * shape with a fixed instruction; the same instruction also drives the
 * post-dictation transform and Scratchpad transforms.
 */
export interface Prompt {
  /** Stable identity (a UUID); the shortcut resolves the instruction by this. */
  id: string;
  name: string;
  /** Instruction sent to the active profile alongside the selection. */
  instruction: string;
  /** Accelerator that applies it; empty = not yet bound (can't fire). */
  shortcut: string;
  /**
   * Shipped by Velata and restored if deleted. User edits to its
   * instruction/shortcut persist; only deletion is undone. Existing custom
   * prompts are user-owned (false).
   */
  builtIn: boolean;
}

export interface Settings {
  /** Schema version for forward migrations. */
  version: number;
  /** Hold-to-talk trigger: press starts, release inserts (default `fn` hold). */
  pushToTalkHotkey: Hotkey;
  /** Hands-free trigger: one press starts, the next stops (default `fn` double-tap). */
  handsFreeHotkey: Hotkey;
  /** Reveals the word-level diff of the last result. Empty `key` disables it. */
  seeChangesHotkey: Hotkey;
  /** Active profile id (a file under `<app-data>/profiles/`); "" = no AI. */
  activeLlmProfileId: string;
  dictionary: DictionaryEntry[];
  /** Spoken shorthands expanded into longer blocks on insert (dictation only). */
  snippets: Snippet[];
  /** Named, shortcut-bound prompts applied to a selection; includes Polish. */
  prompts: Prompt[];
  /**
   * Id of the prompt to run automatically on the transcript after dictation, or
   * null for no post-dictation transform (insert the plain transcript).
   */
  postDictationTransformId: string | null;
  /** Whisper model id from the model registry, e.g. `base.en`. */
  sttModelId: string;
  /** ISO 639-1 code or `auto`. */
  language: string;
  /**
   * Input device to record from, matched by exact name; null = system default.
   * A saved name no longer present falls back to the default.
   */
  inputDeviceName: string | null;
  launchAtLogin: boolean;
  /** Window theme override; `system` defers to macOS. */
  appearance: Appearance;
  /** Opt-in: keep a local, searchable log of past dictations (default off). */
  historyEnabled: boolean;
  /** Days a history entry is kept before purge; 0 = keep forever. */
  historyRetentionDays: number;
  /** STT profile ids whose 'audio leaves the Mac' consent the user confirmed. */
  confirmedSttProfiles: string[];
  /** Master switch for one-time feature tips. */
  tipsEnabled: boolean;
  /** Tip ids already shown; never re-shown. */
  tipsSeen: string[];
  /** Successful dictations ever — the only tip counter (never a log). */
  dictationCount: number;
  /** ISO date (`YYYY-MM-DD`) of the last tip shown. Read and written by the settings webview, which caps its tips at one per day. */
  lastTipShownAt: string;
  /** Keep a Dock icon (vs menu-bar-only). */
  showInDock: boolean;
  /**
   * Opt-in: the Scratchpad notes surface (default off). Off, no note is
   * written and every note command refuses — notes are stored only when on.
   */
  scratchpadEnabled: boolean;
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
   * the transcript for dictation, the original selection for a prompt transform.
   */
  original: string;
  /** Final text that was inserted. */
  text: string;
  /** Whether a prompt transform ran (false means the plain transcript). */
  polished: boolean;
  durationMs: number;
}

/** One entry in the opt-in local dictation history (text only, never audio). */
export interface HistoryEntry {
  id: string;
  /** Unix epoch milliseconds. */
  at: number;
  text: string;
  rawText: string;
  /** Frontmost app's display name at dictation time; null for legacy imports. */
  appName: string | null;
  /** Recording duration in milliseconds; null for legacy imports. */
  durationMs: number | null;
  wordCount: number;
  /** Whether an LLM pass ran (vs rules-based cleanup only). */
  usedAi: boolean;
}

/**
 * One Scratchpad note (text only, never audio). The body is the minimal HTML
 * the editor toolbar produces; paste is forced to plain text so stored markup
 * is bounded to our own tags. Mirrors `Note` in `notes.rs`.
 */
export interface Note {
  id: string;
  title: string;
  /** Body as minimal HTML (`<p>`, `<b>`, `<i>`, `<u>`, `<code>`, lists). */
  content: string;
  /** Unix epoch milliseconds. */
  createdAt: number;
  updatedAt: number;
  pinned: boolean;
}

/** A note row for the list view. Mirrors `NoteSummary` in `notes.rs`. */
export interface NoteSummary {
  id: string;
  title: string;
  /** First ~120 characters of the body with tags stripped. */
  preview: string;
  updatedAt: number;
  pinned: boolean;
}

/**
 * An immutable snapshot of a note's body, taken before a destructive edit
 * (transform or restore). Mirrors `NoteVersion` in `notes.rs`.
 */
export interface NoteVersion {
  id: string;
  noteId: string;
  content: string;
  /** Why it exists: 'created', 'transform', or 'restore'. */
  source: string;
  /** The settings transform applied (when `source` is 'transform'); else null. */
  transformId: string | null;
  createdAt: number;
}

/**
 * Lifetime usage aggregates for the Home header, always kept (counts and dates
 * only, never transcripts or audio) and derived from `insights_daily`. There is
 * no enable toggle and no reset; an empty store reads as all-zero. Mirrors the
 * Rust `Insights` in `stats.rs`.
 */
export interface Insights {
  words: number;
  dictations: number;
  /** Lifetime speaking pace (words ÷ minutes spoken); 0 with no duration. */
  wordsPerMinute: number;
  /** Current consecutive-day dictation streak, in days. */
  streak: number;
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
  /** Fired once a history append has committed; views refresh from durable rows. */
  historyChanged: 'history-changed',
  /** Fired once the insights_daily upsert has committed; the Home header refetches. */
  insightsChanged: 'insights-changed',
  /** A note was created/updated/pinned/deleted/restored/transformed; refresh the list. */
  notesChanged: 'notes-changed',
  /** Ask an open Scratchpad to switch to a note; payload is the note id. */
  scratchpadOpenNote: 'scratchpad-open-note',
  /** Ask the open Settings window to switch to a tab; payload is the tab id. */
  settingsNavigate: 'settings-navigate',
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
  setPostDictationTransform: 'set_post_dictation_transform',
  getLastResult: 'get_last_result',
  getHistory: 'get_history',
  clearHistory: 'clear_history',
  deleteHistoryEntry: 'delete_history_entry',
  getInsights: 'get_insights',
  listDictionarySuggestions: 'list_dictionary_suggestions',
  dismissDictionarySuggestion: 'dismiss_dictionary_suggestion',
  copyText: 'copy_text',
  setChangesInteractive: 'set_changes_interactive',
  setHudMenuOpen: 'set_hud_menu_open',
  testLlm: 'test_llm',
  listLlmProfiles: 'list_llm_profiles',
  saveLlmProfile: 'save_llm_profile',
  deleteLlmProfile: 'delete_llm_profile',
  revealLlmProfiles: 'reveal_llm_profiles',
  listSttProfiles: 'list_stt_profiles',
  saveSttProfile: 'save_stt_profile',
  deleteSttProfile: 'delete_stt_profile',
  revealSttProfiles: 'reveal_stt_profiles',
  exportDictionary: 'export_dictionary',
  listOllamaModels: 'list_ollama_models',
  listInputDevices: 'list_input_devices',
  checkPermissions: 'check_permissions',
  requestMicrophonePermission: 'request_microphone_permission',
  promptAccessibilityPermission: 'prompt_accessibility_permission',
  openAccessibilitySettings: 'open_accessibility_settings',
  openMicrophoneSettings: 'open_microphone_settings',
  getAppInfo: 'get_app_info',
  listNotes: 'list_notes',
  getNote: 'get_note',
  createNote: 'create_note',
  updateNote: 'update_note',
  setNotePinned: 'set_note_pinned',
  deleteNote: 'delete_note',
  listNoteVersions: 'list_note_versions',
  restoreNoteVersion: 'restore_note_version',
  transformNoteText: 'transform_note_text',
  openScratchpadWindow: 'open_scratchpad_window',
  openMainWindow: 'open_main_window',
  openSettingsWindow: 'open_settings_window',
} as const;
