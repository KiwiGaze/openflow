//! Settings persistence: a single `settings.json` under the app config dir.
//!
//! Serialized with `camelCase` field names — the TypeScript mirror lives in
//! `packages/core/src/types.ts`. Update both sides together.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::models::DEFAULT_STT_MODEL_ID;
use crate::prompts;

pub const SETTINGS_VERSION: u32 = 6;
pub const MAX_RECORDING_SECS: u64 = 300;

/// Emitted (with the full `Settings` payload) after every backend-initiated
/// settings change so open webviews stay in sync. Mirrored as
/// `EVENTS.settingsChanged` in `@velata/core`.
pub const SETTINGS_CHANGED_EVENT: &str = "settings-changed";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HotkeyBehavior {
    Hold,
    Toggle,
}

/// Window theme. `System` follows macOS; `Light`/`Dark` force it for Velata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Appearance {
    #[default]
    System,
    Light,
    Dark,
}

/// How synthesized text reaches the active app. Velata always pastes from
/// dictation; the tray's "copy last" uses `Clipboard`. Lives in Rust only —
/// `output.rs`/`tray.rs` pass fixed values, so it never crosses IPC.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertMethod {
    Paste,
    Clipboard,
}

/// v1 embedded the LLM connection inline; kept deserialize-only so the
/// one-time migration into a profile file (`profiles::reconcile`) can read
/// it. Never serialized — persisting settings erases it from disk.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LegacyLlmConfig {
    /// v1 wrote "none" | "ollama" | "openaiCompatible".
    pub provider: String,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub timeout_secs: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DictionaryEntry {
    pub from: String,
    pub to: String,
}

/// A spoken shorthand that expands into a longer block on insert. Unlike a
/// dictionary entry (which fixes a misheard word) a snippet is intentional
/// abbreviation: short trigger → long, possibly multi-line, verbatim text.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snippet {
    /// The spoken phrase that triggers expansion, e.g. "my email".
    pub trigger: String,
    /// Text inserted in place of the trigger; may span multiple lines.
    pub expansion: String,
    /// When true, expand only if the trigger is the whole dictation — for
    /// triggers that also occur in ordinary prose ("my email").
    pub whole_utterance: bool,
}

/// A named, one-tap text operation applied to a selection — a saved instruction
/// with its own shortcut (Transforms page). Polish is the built-in default of
/// the same shape; a custom prompt just carries a user-chosen instruction. The
/// same instruction also drives the post-dictation transform and Scratchpad
/// transforms.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Prompt {
    /// Stable identity (a UUID from the UI); the shortcut handler and the
    /// post-dictation transform look the instruction up by this, so edits take
    /// effect without re-binding.
    pub id: String,
    pub name: String,
    /// Instruction sent to the active profile alongside the selection.
    pub instruction: String,
    /// Accelerator that applies it to the selection; empty = not yet bound
    /// (the prompt exists but can't fire until the user assigns a key).
    pub shortcut: String,
    /// Shipped by Velata and restored by `normalize()` if deleted. User edits to
    /// its instruction/shortcut persist; only its deletion is undone. Old files
    /// default this to false (every existing custom prompt is user-owned).
    #[serde(default)]
    pub built_in: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub version: u32,
    pub dictation_hotkey: String,
    pub dictation_hotkey_behavior: HotkeyBehavior,
    /// Reveals the word-level diff of the last result. "" disables it.
    pub change_overlay_hotkey: String,
    /// Active profile id (a file under `<app-data>/profiles/`); "" = no AI.
    pub active_llm_profile_id: String,
    pub dictionary: Vec<DictionaryEntry>,
    pub snippets: Vec<Snippet>,
    /// Named, shortcut-bound instructions applied to a selection; includes the
    /// built-in Polish. Default = [Polish].
    pub prompts: Vec<Prompt>,
    /// Id of the prompt to run automatically on the transcript after dictation,
    /// or None for no post-dictation transform (insert the plain transcript).
    /// Set via the HUD circle; runs through the same selection-rewrite path.
    pub post_dictation_transform_id: Option<String>,
    pub stt_model_id: String,
    pub language: String,
    /// Input device to record from, matched by exact name; None = system
    /// default. A saved name that is no longer present falls back to the
    /// default so dictation never fails because a mic was unplugged.
    pub input_device_name: Option<String>,
    /// v1 migration only — see `profiles::reconcile`.
    #[serde(skip_serializing)]
    pub llm: Option<LegacyLlmConfig>,
    pub launch_at_login: bool,
    pub appearance: Appearance,
    // ---- Tip system (05); all additive, defaulted via the container default ----
    /// Master switch for one-time feature tips.
    pub tips_enabled: bool,
    /// Tip ids already shown; never re-shown.
    pub tips_seen: Vec<String>,
    /// Successful dictations ever — the only tip-system counter (never a log).
    pub dictation_count: u64,
    /// ISO date (`YYYY-MM-DD`) of the last tip shown. Read and written by the
    /// settings webview, which caps its tips at one per day.
    pub last_tip_shown_at: String,
    /// Opt-in (default off): keep a local, searchable log of past dictations.
    /// Off preserves the no-transcript-persistence privacy default.
    pub history_enabled: bool,
    /// Days a history entry is kept before it is purged; 0 = keep forever.
    /// Enforced on every append and once at startup, on top of the row cap.
    pub history_retention_days: u32,
    /// STT profile ids whose "audio leaves the Mac" consent the user confirmed
    /// (08 §3.2). A profile id only reaches the cloud path once it is in here;
    /// a new endpoint/key (new id) re-confirms.
    pub confirmed_stt_profiles: Vec<String>,
    /// Keep a Dock icon (Regular activation). Off = menu-bar-only (Accessory).
    pub show_in_dock: bool,
    /// Opt-in (default off): the Scratchpad notes surface. Off, no note is
    /// written and every note command refuses — notes are stored only when the
    /// user turns this on.
    pub scratchpad_enabled: bool,
    pub onboarding_completed: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: SETTINGS_VERSION,
            dictation_hotkey: "Alt+Space".into(),
            dictation_hotkey_behavior: HotkeyBehavior::Hold,
            change_overlay_hotkey: "Alt+O".into(),
            active_llm_profile_id: String::new(),
            dictionary: Vec::new(),
            snippets: Vec::new(),
            prompts: prompts::built_in_prompts(),
            post_dictation_transform_id: None,
            stt_model_id: DEFAULT_STT_MODEL_ID.into(),
            language: "auto".into(),
            input_device_name: None,
            llm: None,
            launch_at_login: false,
            appearance: Appearance::System,
            tips_enabled: true,
            tips_seen: Vec::new(),
            dictation_count: 0,
            last_tip_shown_at: String::new(),
            history_enabled: false,
            history_retention_days: 0,
            confirmed_stt_profiles: Vec::new(),
            show_in_dock: false,
            scratchpad_enabled: false,
            onboarding_completed: false,
        }
    }
}

impl Settings {
    /// Repairs invariants instead of rejecting input: the built-in Polish prompt
    /// is restored if deleted, and structurally-invalid rows are dropped.
    fn normalize(&mut self) {
        self.dictionary
            .retain(|e| !e.from.trim().is_empty() && !e.to.trim().is_empty());
        self.snippets
            .retain(|s| !s.trigger.trim().is_empty() && !s.expansion.is_empty());
        // Prompts are created and removed explicitly in the UI, which saves on
        // every keystroke — so never drop one because a field is transiently
        // blank mid-edit (that would lose its instruction and shortcut while the
        // user renames it). Only drop structurally-invalid rows with no id.
        self.prompts.retain(|p| !p.id.trim().is_empty());
        // Built-in prompts are restored if deleted, but an existing copy is left
        // untouched so the user's edits to its instruction or shortcut persist —
        // only deletion is undone. An empty-string shortcut is the "unbound"
        // sentinel (kept as-is; the registrar skips blanks), so no normalization
        // is needed there.
        for built_in in prompts::built_in_prompts() {
            if !self.prompts.iter().any(|p| p.id == built_in.id) {
                self.prompts.push(built_in);
            }
        }
        // A post-dictation transform pointing at a deleted prompt is cleared, so
        // a dangling id can't silently disable the transform — None and "no such
        // prompt" both mean "insert the plain transcript".
        if let Some(id) = &self.post_dictation_transform_id {
            if id.trim().is_empty() || !self.prompts.iter().any(|p| &p.id == id) {
                self.post_dictation_transform_id = None;
            }
        }
        self.version = SETTINGS_VERSION;
    }
}

pub struct SettingsManager {
    path: PathBuf,
    current: RwLock<Settings>,
}

impl SettingsManager {
    /// Loads settings from `<config_dir>/settings.json`, falling back to (and
    /// persisting) defaults when the file is missing or unreadable. A corrupt
    /// file is preserved as `settings.json.bak` instead of being overwritten.
    pub fn load(config_dir: &Path) -> Self {
        let path = config_dir.join("settings.json");
        let mut settings = match fs::read_to_string(&path) {
            Ok(raw) => match serde_json::from_str::<Settings>(&raw) {
                Ok(s) => s,
                Err(err) => {
                    log::warn!(
                        "settings.json is corrupt ({err}); backing it up and using defaults"
                    );
                    let _ = fs::rename(&path, path.with_extension("json.bak"));
                    Settings::default()
                }
            },
            Err(_) => Settings::default(),
        };
        settings.normalize();
        let manager = Self {
            path,
            current: RwLock::new(settings),
        };
        // Defer the rewrite while a v1 LLM block is present: persisting now
        // would erase it (the field is deserialize-only) before
        // `profiles::reconcile` migrates it into a profile file.
        let loaded = manager.get();
        if loaded.llm.is_none() {
            if let Err(err) = manager.persist(&loaded) {
                log::warn!("could not persist settings on load: {err}");
            }
        }
        manager
    }

    pub fn get(&self) -> Settings {
        self.current.read().expect("settings lock poisoned").clone()
    }

    pub fn set(&self, mut settings: Settings) -> AppResult<Settings> {
        settings.normalize();
        {
            let mut guard = self.current.write().expect("settings lock poisoned");
            *guard = settings.clone();
        }
        self.persist(&settings)?;
        Ok(settings)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Atomic write: serialize to a sibling temp file, then rename over the
    /// target so a crash can never leave a half-written settings file.
    fn persist(&self, settings: &Settings) -> AppResult<()> {
        let json = serde_json::to_string_pretty(settings)
            .map_err(|e| AppError::Settings(e.to_string()))?;
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir)?;
        }
        let tmp = self.path.with_extension("json.tmp");
        fs::write(&tmp, json)?;
        fs::rename(&tmp, &self.path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("velata-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn defaults_are_created_and_persisted() {
        let dir = temp_dir("defaults");
        let manager = SettingsManager::load(&dir);
        assert!(manager.path().exists());
        let s = manager.get();
        assert_eq!(s.dictation_hotkey, "Alt+Space");
        assert!(s.prompts.iter().any(|p| p.id == prompts::POLISH_PROMPT_ID));
        assert_eq!(s.post_dictation_transform_id, None);
    }

    #[test]
    fn roundtrip_preserves_values() {
        let dir = temp_dir("roundtrip");
        let manager = SettingsManager::load(&dir);
        let mut s = manager.get();
        s.dictation_hotkey = "F5".into();
        s.dictionary.push(DictionaryEntry {
            from: "open flow".into(),
            to: "Velata".into(),
        });
        manager.set(s).unwrap();

        let reloaded = SettingsManager::load(&dir).get();
        assert_eq!(reloaded.dictation_hotkey, "F5");
        assert_eq!(reloaded.dictionary.len(), 1);
    }

    #[test]
    fn corrupt_file_falls_back_to_defaults_with_backup() {
        let dir = temp_dir("corrupt");
        fs::write(dir.join("settings.json"), "{ not json").unwrap();
        let manager = SettingsManager::load(&dir);
        assert_eq!(manager.get().version, SETTINGS_VERSION);
        assert!(dir.join("settings.json.bak").exists());
    }

    #[test]
    fn unknown_fields_are_tolerated_and_missing_fields_defaulted() {
        // Old settings carried mode/cleanup/polish fields this version no longer
        // knows; serde tolerates the unknown keys, and the missing new ones
        // (prompts, postDictationTransformId) default rather than fail the load.
        let dir = temp_dir("forward-compat");
        fs::write(
            dir.join("settings.json"),
            r#"{ "version": 1, "dictationHotkey": "F6", "activeModeId": "standard",
                 "autoCleanupLevel": "rules", "someFutureField": true }"#,
        )
        .unwrap();
        let s = SettingsManager::load(&dir).get();
        assert_eq!(s.dictation_hotkey, "F6");
        assert!(s.prompts.iter().any(|p| p.id == prompts::POLISH_PROMPT_ID));
        assert_eq!(s.post_dictation_transform_id, None);
    }

    #[test]
    fn serializes_with_camel_case_contract() {
        let json = serde_json::to_string(&Settings::default()).unwrap();
        assert!(json.contains("\"dictationHotkey\""));
        assert!(json.contains("\"changeOverlayHotkey\":\"Alt+O\""));
        assert!(json.contains("\"activeLlmProfileId\":\"\""));
        assert!(json.contains("\"appearance\":\"system\""));
        assert!(json.contains("\"tipsEnabled\":true"));
        assert!(json.contains("\"dictationCount\":0"));
        assert!(json.contains("\"historyEnabled\":false"));
        assert!(json.contains("\"historyRetentionDays\":0"));
        assert!(json.contains("\"inputDeviceName\":null"));
        assert!(json.contains("\"snippets\":[]"));
        assert!(json.contains("\"showInDock\":false"));
        assert!(json.contains("\"scratchpadEnabled\":false"));
        // The post-dictation transform is unset by default.
        assert!(json.contains("\"postDictationTransformId\":null"));
        // The seeded Polish prompt serializes camelCase with the `builtIn` flag
        // and a `shortcut` (not the old `hotkey`).
        assert!(json.contains("\"prompts\":[{"));
        assert!(json.contains("\"shortcut\":\"Alt+Shift+P\""));
        assert!(json.contains("\"builtIn\":true"));
        // Removed keys must not reappear.
        assert!(!json.contains("\"activeModeId\""));
        assert!(!json.contains("\"polishHotkey\""));
        assert!(!json.contains("\"polishAfterDictation\""));
        assert!(!json.contains("\"insertMethod\""));
        assert!(!json.contains("\"restoreClipboard\""));
        assert!(!json.contains("\"autoCleanupLevel\""));
        assert!(!json.contains("\"polishRules\""));
        assert!(!json.contains("\"appRules\""));
        assert!(!json.contains("\"modes\""));
        // The v1 LLM block is deserialize-only; it must never be written.
        assert!(!json.contains("\"llm\""));
    }

    #[test]
    fn normalize_clears_a_dangling_post_dictation_transform_id() {
        let dir = temp_dir("post-dictation-dangling");
        let manager = SettingsManager::load(&dir);

        // An id pointing at the built-in Polish survives.
        let mut s = manager.get();
        s.post_dictation_transform_id = Some(prompts::POLISH_PROMPT_ID.into());
        let kept = manager.set(s).unwrap();
        assert_eq!(
            kept.post_dictation_transform_id.as_deref(),
            Some(prompts::POLISH_PROMPT_ID)
        );

        // An id with no matching prompt is cleared rather than silently kept.
        let mut s = manager.get();
        s.post_dictation_transform_id = Some("does-not-exist".into());
        let cleared = manager.set(s).unwrap();
        assert_eq!(cleared.post_dictation_transform_id, None);
    }

    #[test]
    fn retention_and_input_device_default_for_old_files() {
        // A file predating this task has neither field; both must default
        // (retention 0 = keep forever, device None = system default) rather
        // than fail the load.
        let dir = temp_dir("retention-device-defaults");
        fs::write(
            dir.join("settings.json"),
            r#"{ "version": 5, "historyEnabled": true }"#,
        )
        .unwrap();
        let s = SettingsManager::load(&dir).get();
        assert_eq!(s.history_retention_days, 0);
        assert_eq!(s.input_device_name, None);

        // An explicit pair round-trips through load and reaches both fields.
        let dir = temp_dir("retention-device-explicit");
        fs::write(
            dir.join("settings.json"),
            r#"{ "version": 5, "historyRetentionDays": 30, "inputDeviceName": "MacBook Pro Microphone" }"#,
        )
        .unwrap();
        let s = SettingsManager::load(&dir).get();
        assert_eq!(s.history_retention_days, 30);
        assert_eq!(
            s.input_device_name.as_deref(),
            Some("MacBook Pro Microphone")
        );
    }

    #[test]
    fn normalize_drops_blank_snippets_and_keeps_valid_ones() {
        let dir = temp_dir("snippets");
        let manager = SettingsManager::load(&dir);
        let mut s = manager.get();
        s.snippets = vec![
            Snippet {
                trigger: "my email".into(),
                expansion: "me@example.com".into(),
                whole_utterance: true,
            },
            Snippet {
                trigger: "  ".into(),
                expansion: "dropped — blank trigger".into(),
                whole_utterance: false,
            },
            Snippet {
                trigger: "empty".into(),
                expansion: String::new(),
                whole_utterance: false,
            },
        ];
        let fixed = manager.set(s).unwrap();
        assert_eq!(fixed.snippets.len(), 1);
        assert_eq!(fixed.snippets[0].trigger, "my email");

        let reloaded = SettingsManager::load(&dir).get();
        assert_eq!(reloaded.snippets.len(), 1);
        assert!(reloaded.snippets[0].whole_utterance);
    }

    #[test]
    fn normalize_keeps_prompts_being_edited_drops_only_idless_rows() {
        let dir = temp_dir("prompts");
        let manager = SettingsManager::load(&dir);
        let mut s = manager.get();
        s.prompts = vec![
            Prompt {
                id: "a".into(),
                name: "Concise".into(),
                instruction: "Tighten the wording.".into(),
                shortcut: "Alt+1".into(),
                built_in: false,
            },
            // Mid-rename: the name is transiently blank but the instruction and
            // shortcut must survive (a keystroke must not delete the prompt).
            Prompt {
                id: "b".into(),
                name: "  ".into(),
                instruction: "Make it friendlier.".into(),
                shortcut: "Alt+2".into(),
                built_in: false,
            },
            // Structurally invalid (no id, e.g. a bad hand-edited file) → dropped.
            Prompt {
                id: " ".into(),
                name: "Orphan".into(),
                instruction: "x".into(),
                shortcut: String::new(),
                built_in: false,
            },
        ];
        let fixed = manager.set(s).unwrap();
        // The two id-bearing customs survive; the built-in Polish is re-added
        // because the cleared list above dropped it.
        let renamed = fixed.prompts.iter().find(|p| p.id == "b").unwrap();
        assert_eq!(renamed.instruction, "Make it friendlier.");
        assert_eq!(renamed.shortcut, "Alt+2");
        assert!(fixed.prompts.iter().any(|p| p.id == "a"));
        assert!(!fixed.prompts.iter().any(|p| p.id.trim().is_empty()));
        assert!(fixed
            .prompts
            .iter()
            .any(|p| p.id == prompts::POLISH_PROMPT_ID && p.built_in));
    }

    #[test]
    fn normalize_restores_deleted_polish_but_keeps_user_edits() {
        let dir = temp_dir("polish-restore");
        let manager = SettingsManager::load(&dir);

        // Deleting it (empty list) brings the built-in back on the next save.
        let mut s = manager.get();
        s.prompts.clear();
        let restored = manager.set(s).unwrap();
        let polish = restored
            .prompts
            .iter()
            .find(|p| p.id == prompts::POLISH_PROMPT_ID)
            .expect("Polish restored after deletion");
        assert!(polish.built_in);

        // Editing its instruction and shortcut must persist — normalize re-adds
        // only when missing, never clobbering an existing built-in.
        let mut s = manager.get();
        for p in &mut s.prompts {
            if p.id == prompts::POLISH_PROMPT_ID {
                p.instruction = "My own polish wording.".into();
                p.shortcut = "Alt+9".into();
            }
        }
        let edited = manager.set(s).unwrap();
        let polish = edited
            .prompts
            .iter()
            .find(|p| p.id == prompts::POLISH_PROMPT_ID)
            .expect("Polish still present after editing");
        assert_eq!(polish.instruction, "My own polish wording.");
        assert_eq!(polish.shortcut, "Alt+9");
        assert!(polish.built_in);
    }

    #[test]
    fn old_settings_without_built_in_flag_load_with_defaults() {
        // A file predating this change has a custom prompt with no `builtIn`
        // field; it must default to false rather than fail the load, and the
        // built-in Polish is appended.
        let dir = temp_dir("prompt-builtin-defaults");
        fs::write(
            dir.join("settings.json"),
            r#"{ "version": 5,
                 "prompts": [{ "id": "c1", "name": "Mine", "instruction": "Tighten it.", "shortcut": "Alt+1" }] }"#,
        )
        .unwrap();
        let s = SettingsManager::load(&dir).get();

        let mine = s.prompts.iter().find(|p| p.id == "c1").unwrap();
        assert!(!mine.built_in);
        assert!(s
            .prompts
            .iter()
            .any(|p| p.id == prompts::POLISH_PROMPT_ID && p.built_in));
    }

    #[test]
    fn legacy_llm_block_is_read_and_kept_on_disk_until_persisted() {
        let dir = temp_dir("legacy-llm");
        fs::write(
            dir.join("settings.json"),
            r#"{ "version": 1, "llm": { "provider": "ollama", "model": "qwen2.5:3b" } }"#,
        )
        .unwrap();
        let manager = SettingsManager::load(&dir);
        let legacy = manager.get().llm.expect("legacy block parsed");
        assert_eq!(legacy.provider, "ollama");
        assert_eq!(legacy.model, "qwen2.5:3b");
        // Load deferred the rewrite, so migration can still read the file.
        let raw = fs::read_to_string(manager.path()).unwrap();
        assert!(raw.contains("\"llm\""));

        let mut s = manager.get();
        s.llm = None;
        manager.set(s).unwrap();
        let raw = fs::read_to_string(manager.path()).unwrap();
        assert!(!raw.contains("\"llm\""));
    }
}
