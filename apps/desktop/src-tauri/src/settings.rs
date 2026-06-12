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
use crate::modes;

pub const SETTINGS_VERSION: u32 = 5;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InsertMethod {
    Paste,
    Clipboard,
}

/// How much the dictation transcript is reshaped before insertion (Style page).
/// `Off` inserts speech verbatim, `Rules` runs the deterministic cleanup, `Ai`
/// keeps each mode's own behavior (LLM when the mode uses it, else rules). It is
/// a processing dial layered over the mode, not a second mode concept.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CleanupLevel {
    Off,
    Rules,
    #[default]
    Ai,
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
pub struct Mode {
    pub id: String,
    pub name: String,
    pub built_in: bool,
    pub uses_llm: bool,
    /// When true the appended default "preserve the language" line is dropped,
    /// so the mode may translate/re-cast (still fenced by SAFETY_RULES). The
    /// invariant rules always apply. Old files default this to false.
    #[serde(default)]
    pub transforms: bool,
    pub prompt: String,
    // ---- Mode v2 overrides (07); null = inherit the global setting ----
    /// AI profile id, or null to use the globally active profile.
    #[serde(default)]
    pub ai_profile_id: Option<String>,
    /// Whisper model id, or null to use the global speech model.
    #[serde(default)]
    pub stt_model_id: Option<String>,
    /// ISO 639-1 code or `auto`, or null to use the global spoken language.
    #[serde(default)]
    pub language: Option<String>,
    /// Per-mode one-shot hotkey accelerator, or null for none.
    #[serde(default)]
    pub hotkey: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DictionaryEntry {
    pub from: String,
    pub to: String,
}

/// A per-app rule (07 §9): dictating while `bundle_id` is frontmost uses
/// `mode_id` for that job only, like a mode hotkey — the active mode is unchanged.
/// A rule always names a mode; `cleanup_level` optionally overrides the global
/// auto-cleanup level for this app (None = inherit `Settings.auto_cleanup_level`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppRule {
    pub bundle_id: String,
    pub mode_id: String,
    #[serde(default)]
    pub cleanup_level: Option<CleanupLevel>,
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

/// A named, one-tap text operation applied to the current selection — a saved
/// Rewrite instruction with its own hotkey. Polish is the built-in default of
/// the same shape; a transform just carries a user-chosen instruction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transform {
    /// Stable identity (a UUID from the UI); the hotkey handler looks the
    /// instruction up by this so edits take effect without re-binding.
    pub id: String,
    pub name: String,
    /// Instruction sent to the active profile alongside the selection.
    pub instruction: String,
    /// Accelerator that applies it to the selection; empty = not yet bound
    /// (the transform exists but can't fire until the user assigns a key).
    pub hotkey: String,
    /// Shipped by Velata and restored by `normalize()` if deleted (like a
    /// built-in mode). User edits to its instruction/hotkey persist; only its
    /// deletion is undone. Old files default this to false (every existing
    /// custom transform is user-owned).
    #[serde(default)]
    pub built_in: bool,
}

/// Which optional rewrites the built-in Polish composes on top of its always-on
/// grammar/spelling fix (Transforms page). Each flag adds one instruction
/// sentence. Defaults preserve Polish's pre-rules identity — clarity and tone
/// on (≈ the old fix-grammar default instruction), concise and structure as
/// opt-in dials. `polish_instruction` in `modes.rs` turns these into the prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PolishRules {
    pub concise: bool,
    pub clarity: bool,
    pub structure: bool,
    pub tone: bool,
}

impl Default for PolishRules {
    fn default() -> Self {
        Self {
            concise: false,
            clarity: true,
            structure: false,
            tone: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub version: u32,
    pub dictation_hotkey: String,
    pub dictation_hotkey_behavior: HotkeyBehavior,
    pub polish_hotkey: String,
    /// Reveals the word-level diff of the last result. "" disables it.
    pub change_overlay_hotkey: String,
    /// Master switch: may dictation transcripts go to the active profile.
    #[serde(alias = "refineAfterDictation")]
    pub polish_after_dictation: bool,
    /// Active profile id (a file under `<app-data>/profiles/`); "" = no AI.
    pub active_llm_profile_id: String,
    pub active_mode_id: String,
    pub modes: Vec<Mode>,
    pub dictionary: Vec<DictionaryEntry>,
    pub snippets: Vec<Snippet>,
    pub transforms: Vec<Transform>,
    /// Optional rewrites the built-in Polish layers over its grammar fix.
    pub polish_rules: PolishRules,
    pub stt_model_id: String,
    pub language: String,
    /// v1 migration only — see `profiles::reconcile`.
    #[serde(skip_serializing)]
    pub llm: Option<LegacyLlmConfig>,
    pub insert_method: InsertMethod,
    pub restore_clipboard: bool,
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
    /// Opt-in (default off): persist all-time usage counts and dates (never
    /// words or audio) to `insights_daily` for the Insights view's lifetime
    /// totals and streaks. Off keeps insights session-only, in-RAM.
    pub app_stats_enabled: bool,
    /// Per-app rules: dictate in a chosen mode when an app is frontmost (07 §9).
    pub app_rules: Vec<AppRule>,
    /// Global cleanup strength for dictation (Style page). An app rule with a
    /// `cleanup_level` overrides this for that app; default `Ai` = mode decides.
    pub auto_cleanup_level: CleanupLevel,
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
            polish_hotkey: "Alt+Shift+P".into(),
            change_overlay_hotkey: "Alt+O".into(),
            polish_after_dictation: true,
            active_llm_profile_id: String::new(),
            active_mode_id: modes::STANDARD_MODE_ID.into(),
            modes: modes::built_in_modes(),
            dictionary: Vec::new(),
            snippets: Vec::new(),
            transforms: modes::built_in_transforms(),
            polish_rules: PolishRules::default(),
            stt_model_id: DEFAULT_STT_MODEL_ID.into(),
            language: "auto".into(),
            llm: None,
            insert_method: InsertMethod::Paste,
            restore_clipboard: true,
            launch_at_login: false,
            appearance: Appearance::System,
            tips_enabled: true,
            tips_seen: Vec::new(),
            dictation_count: 0,
            last_tip_shown_at: String::new(),
            history_enabled: false,
            app_stats_enabled: false,
            app_rules: Vec::new(),
            auto_cleanup_level: CleanupLevel::Ai,
            confirmed_stt_profiles: Vec::new(),
            show_in_dock: false,
            scratchpad_enabled: false,
            onboarding_completed: false,
        }
    }
}

impl Settings {
    pub fn active_mode(&self) -> Mode {
        self.modes
            .iter()
            .find(|m| m.id == self.active_mode_id)
            .cloned()
            .unwrap_or_else(|| modes::built_in_modes().remove(0))
    }

    /// Repairs invariants instead of rejecting input: built-in modes are
    /// restored if deleted and the active mode id must point somewhere.
    fn normalize(&mut self) {
        // Built-in modes are read-only, so refresh persisted copies in place —
        // this is how prompt/flag changes in code (e.g. the SAFETY_RULES split)
        // reach existing installs — and re-add any that were deleted.
        for built_in in modes::built_in_modes() {
            match self.modes.iter_mut().find(|m| m.id == built_in.id) {
                Some(existing) => *existing = built_in,
                None => self.modes.push(built_in),
            }
        }
        if !self.modes.iter().any(|m| m.id == self.active_mode_id) {
            self.active_mode_id = modes::STANDARD_MODE_ID.into();
        }
        // An empty-string mode hotkey normalizes to None so "" never reaches
        // the registrar (built-ins carry no overrides — refreshed above).
        for mode in &mut self.modes {
            if mode.hotkey.as_deref() == Some("") {
                mode.hotkey = None;
            }
        }
        self.dictionary
            .retain(|e| !e.from.trim().is_empty() && !e.to.trim().is_empty());
        self.snippets
            .retain(|s| !s.trigger.trim().is_empty() && !s.expansion.is_empty());
        // Transforms are created and removed explicitly in the UI, which saves
        // on every keystroke — so never drop one because a field is transiently
        // blank mid-edit (that would lose its instruction and hotkey while the
        // user renames it). Only drop structurally-invalid rows with no id.
        self.transforms.retain(|t| !t.id.trim().is_empty());
        // Built-in transforms are restored if deleted (the built-in-modes
        // pattern). Unlike modes, an existing copy is left untouched so the
        // user's edits to its instruction or hotkey persist — only deletion is
        // undone. An empty-string transform hotkey is the "unbound" sentinel
        // (kept as-is; the registrar already skips blanks), so no normalization
        // is needed there.
        for built_in in modes::built_in_transforms() {
            if !self.transforms.iter().any(|t| t.id == built_in.id) {
                self.transforms.push(built_in);
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
        assert!(!s.modes.is_empty());
        assert_eq!(s.active_mode_id, modes::STANDARD_MODE_ID);
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
    fn normalize_restores_built_in_modes_and_active_id() {
        let dir = temp_dir("normalize");
        let manager = SettingsManager::load(&dir);
        let mut s = manager.get();
        s.modes.clear();
        s.active_mode_id = "missing".into();
        let fixed = manager.set(s).unwrap();
        assert!(fixed.modes.iter().any(|m| m.id == modes::STANDARD_MODE_ID));
        assert_eq!(fixed.active_mode_id, modes::STANDARD_MODE_ID);
    }

    #[test]
    fn legacy_refine_after_dictation_key_still_reads() {
        let dir = temp_dir("polish-alias");
        fs::write(
            dir.join("settings.json"),
            r#"{ "version": 3, "refineAfterDictation": false }"#,
        )
        .unwrap();
        let manager = SettingsManager::load(&dir);
        let s = manager.get();
        // Default is true, so false proves the alias carried the old value.
        assert!(!s.polish_after_dictation);
        assert_eq!(s.version, SETTINGS_VERSION);
        // The load rewrote the file under the new key only.
        let raw = fs::read_to_string(manager.path()).unwrap();
        assert!(raw.contains("\"polishAfterDictation\": false"));
        assert!(!raw.contains("refineAfterDictation"));
    }

    #[test]
    fn unknown_fields_are_tolerated_and_missing_fields_defaulted() {
        let dir = temp_dir("forward-compat");
        fs::write(
            dir.join("settings.json"),
            r#"{ "version": 1, "dictationHotkey": "F6", "someFutureField": true }"#,
        )
        .unwrap();
        let s = SettingsManager::load(&dir).get();
        assert_eq!(s.dictation_hotkey, "F6");
        assert_eq!(s.insert_method, InsertMethod::Paste);
    }

    #[test]
    fn serializes_with_camel_case_contract() {
        let json = serde_json::to_string(&Settings::default()).unwrap();
        assert!(json.contains("\"dictationHotkey\""));
        assert!(json.contains("\"activeModeId\""));
        assert!(json.contains("\"polishHotkey\":\"Alt+Shift+P\""));
        assert!(json.contains("\"changeOverlayHotkey\":\"Alt+O\""));
        assert!(json.contains("\"polishAfterDictation\":true"));
        assert!(json.contains("\"activeLlmProfileId\":\"\""));
        assert!(json.contains("\"insertMethod\":\"paste\""));
        assert!(json.contains("\"appearance\":\"system\""));
        assert!(json.contains("\"tipsEnabled\":true"));
        assert!(json.contains("\"dictationCount\":0"));
        assert!(json.contains("\"historyEnabled\":false"));
        assert!(json.contains("\"appStatsEnabled\":false"));
        assert!(json.contains("\"snippets\":[]"));
        assert!(json.contains("\"showInDock\":false"));
        assert!(json.contains("\"scratchpadEnabled\":false"));
        assert!(json.contains("\"autoCleanupLevel\":\"ai\""));
        // Polish rules serialize camelCase; the defaults keep Polish's
        // pre-rules identity (clarity + tone on, concise + structure opt-in).
        // The seeded Prompt Engineer carries the `builtIn` flag.
        assert!(json.contains(
            "\"polishRules\":{\"concise\":false,\"clarity\":true,\"structure\":false,\"tone\":true}"
        ));
        assert!(json.contains("\"builtIn\":true"));
        // The v1 LLM block is deserialize-only; it must never be written.
        assert!(!json.contains("\"llm\""));
    }

    #[test]
    fn cleanup_level_loads_with_and_without_the_new_fields() {
        // A file predating the Style page omits both new fields; they must
        // default (global = Ai, rule level = None) rather than fail the load.
        let dir = temp_dir("cleanup-defaults");
        fs::write(
            dir.join("settings.json"),
            r#"{ "version": 5, "appRules": [{ "bundleId": "com.apple.Notes", "modeId": "literal" }] }"#,
        )
        .unwrap();
        let s = SettingsManager::load(&dir).get();
        assert_eq!(s.auto_cleanup_level, CleanupLevel::Ai);
        assert_eq!(s.app_rules.len(), 1);
        assert_eq!(s.app_rules[0].cleanup_level, None);

        // A file that sets both round-trips through load and reaches the field.
        let dir = temp_dir("cleanup-explicit");
        fs::write(
            dir.join("settings.json"),
            r#"{ "version": 5, "autoCleanupLevel": "rules",
                 "appRules": [{ "bundleId": "com.apple.Terminal", "modeId": "code", "cleanupLevel": "off" }] }"#,
        )
        .unwrap();
        let s = SettingsManager::load(&dir).get();
        assert_eq!(s.auto_cleanup_level, CleanupLevel::Rules);
        assert_eq!(s.app_rules[0].cleanup_level, Some(CleanupLevel::Off));
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
    fn normalize_keeps_transforms_being_edited_drops_only_idless_rows() {
        let dir = temp_dir("transforms");
        let manager = SettingsManager::load(&dir);
        let mut s = manager.get();
        s.transforms = vec![
            Transform {
                id: "a".into(),
                name: "Concise".into(),
                instruction: "Tighten the wording.".into(),
                hotkey: "Alt+1".into(),
                built_in: false,
            },
            // Mid-rename: the name is transiently blank but the instruction and
            // hotkey must survive (a keystroke must not delete the transform).
            Transform {
                id: "b".into(),
                name: "  ".into(),
                instruction: "Make it friendlier.".into(),
                hotkey: "Alt+2".into(),
                built_in: false,
            },
            // Structurally invalid (no id, e.g. a bad hand-edited file) → dropped.
            Transform {
                id: " ".into(),
                name: "Orphan".into(),
                instruction: "x".into(),
                hotkey: String::new(),
                built_in: false,
            },
        ];
        let fixed = manager.set(s).unwrap();
        // The two id-bearing customs survive; the built-in Prompt Engineer is
        // re-added because the cleared list above dropped it.
        let renamed = fixed.transforms.iter().find(|t| t.id == "b").unwrap();
        assert_eq!(renamed.instruction, "Make it friendlier.");
        assert_eq!(renamed.hotkey, "Alt+2");
        assert!(fixed.transforms.iter().any(|t| t.id == "a"));
        assert!(!fixed.transforms.iter().any(|t| t.id.trim().is_empty()));
        assert!(fixed
            .transforms
            .iter()
            .any(|t| t.id == modes::PROMPT_ENGINEER_TRANSFORM_ID && t.built_in));
    }

    #[test]
    fn normalize_restores_deleted_prompt_engineer_but_keeps_user_edits() {
        let dir = temp_dir("prompt-engineer-restore");
        let manager = SettingsManager::load(&dir);

        // Deleting it (empty list) brings the built-in back on the next save.
        let mut s = manager.get();
        s.transforms.clear();
        let restored = manager.set(s).unwrap();
        let pe = restored
            .transforms
            .iter()
            .find(|t| t.id == modes::PROMPT_ENGINEER_TRANSFORM_ID)
            .expect("Prompt Engineer restored after deletion");
        assert!(pe.built_in);

        // Editing its instruction and hotkey must persist — normalize re-adds
        // only when missing, never clobbering an existing built-in.
        let mut s = manager.get();
        for t in &mut s.transforms {
            if t.id == modes::PROMPT_ENGINEER_TRANSFORM_ID {
                t.instruction = "My own prompt wording.".into();
                t.hotkey = "Alt+9".into();
            }
        }
        let edited = manager.set(s).unwrap();
        let pe = edited
            .transforms
            .iter()
            .find(|t| t.id == modes::PROMPT_ENGINEER_TRANSFORM_ID)
            .expect("Prompt Engineer still present after editing");
        assert_eq!(pe.instruction, "My own prompt wording.");
        assert_eq!(pe.hotkey, "Alt+9");
        assert!(pe.built_in);
    }

    #[test]
    fn old_settings_without_polish_rules_or_built_in_load_with_defaults() {
        // A file predating this change has no `polishRules` and a custom
        // transform with no `builtIn` field. Both must default rather than
        // fail the load, and the built-in Prompt Engineer is appended.
        let dir = temp_dir("polish-rules-defaults");
        fs::write(
            dir.join("settings.json"),
            r#"{ "version": 5,
                 "transforms": [{ "id": "c1", "name": "Mine", "instruction": "Tighten it.", "hotkey": "Alt+1" }] }"#,
        )
        .unwrap();
        let s = SettingsManager::load(&dir).get();

        assert_eq!(s.polish_rules, PolishRules::default());
        // The defaults match Polish's old behavior: clarity + tone on,
        // concise + structure off until the user opts in.
        assert!(s.polish_rules.clarity && s.polish_rules.tone);
        assert!(!s.polish_rules.concise && !s.polish_rules.structure);

        let mine = s.transforms.iter().find(|t| t.id == "c1").unwrap();
        assert!(!mine.built_in);
        assert!(s
            .transforms
            .iter()
            .any(|t| t.id == modes::PROMPT_ENGINEER_TRANSFORM_ID && t.built_in));
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
