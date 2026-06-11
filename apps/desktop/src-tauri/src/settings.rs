//! Settings persistence: a single `settings.json` under the app config dir.
//!
//! Serialized with `camelCase` field names — the TypeScript mirror lives in
//! `packages/core/src/types.ts`. Update both sides together.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::modes;

pub const SETTINGS_VERSION: u32 = 3;
pub const MAX_RECORDING_SECS: u64 = 300;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HotkeyBehavior {
    Hold,
    Toggle,
}

/// Window theme. `System` follows macOS; `Light`/`Dark` force it for OpenFlow.
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppRule {
    pub bundle_id: String,
    pub mode_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub version: u32,
    pub dictation_hotkey: String,
    pub dictation_hotkey_behavior: HotkeyBehavior,
    pub refine_hotkey: String,
    pub polish_hotkey: String,
    /// Reveals the word-level diff of the last result. "" disables it.
    pub change_overlay_hotkey: String,
    /// Master switch: may dictation transcripts go to the active profile.
    pub refine_after_dictation: bool,
    /// Active profile id (a file under `<app-data>/profiles/`); "" = no AI.
    pub active_llm_profile_id: String,
    pub active_mode_id: String,
    pub modes: Vec<Mode>,
    pub dictionary: Vec<DictionaryEntry>,
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
    /// ISO date (`YYYY-MM-DD`) of the last tip shown; enforces ≤ 1 tip/day.
    pub last_tip_shown_at: String,
    /// Opt-in (default off): keep a local, searchable log of past dictations.
    /// Off preserves the no-transcript-persistence privacy default.
    pub history_enabled: bool,
    /// Per-app rules: dictate in a chosen mode when an app is frontmost (07 §9).
    pub app_rules: Vec<AppRule>,
    /// STT profile ids whose "audio leaves the Mac" consent the user confirmed
    /// (08 §3.2). A profile id only reaches the cloud path once it is in here;
    /// a new endpoint/key (new id) re-confirms.
    pub confirmed_stt_profiles: Vec<String>,
    pub onboarding_completed: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: SETTINGS_VERSION,
            dictation_hotkey: "Alt+Space".into(),
            dictation_hotkey_behavior: HotkeyBehavior::Hold,
            refine_hotkey: "Alt+Shift+Space".into(),
            polish_hotkey: "Alt+Shift+P".into(),
            change_overlay_hotkey: "Alt+O".into(),
            refine_after_dictation: true,
            active_llm_profile_id: String::new(),
            active_mode_id: modes::STANDARD_MODE_ID.into(),
            modes: modes::built_in_modes(),
            dictionary: Vec::new(),
            stt_model_id: "base.en".into(),
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
            app_rules: Vec::new(),
            confirmed_stt_profiles: Vec::new(),
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
        if manager.get().llm.is_none() {
            if let Err(err) = manager.persist() {
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
        self.persist()?;
        Ok(settings)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Atomic write: serialize to a sibling temp file, then rename over the
    /// target so a crash can never leave a half-written settings file.
    fn persist(&self) -> AppResult<()> {
        let settings = self.get();
        let json = serde_json::to_string_pretty(&settings)
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
        let dir = std::env::temp_dir().join(format!("openflow-test-{name}-{}", std::process::id()));
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
            to: "OpenFlow".into(),
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
        assert!(json.contains("\"refineAfterDictation\":true"));
        assert!(json.contains("\"activeLlmProfileId\":\"\""));
        assert!(json.contains("\"insertMethod\":\"paste\""));
        assert!(json.contains("\"appearance\":\"system\""));
        assert!(json.contains("\"tipsEnabled\":true"));
        assert!(json.contains("\"dictationCount\":0"));
        assert!(json.contains("\"historyEnabled\":false"));
        // The v1 LLM block is deserialize-only; it must never be written.
        assert!(!json.contains("\"llm\""));
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
