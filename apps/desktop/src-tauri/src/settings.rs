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

pub const SETTINGS_VERSION: u32 = 1;
pub const MAX_RECORDING_SECS: u64 = 300;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HotkeyBehavior {
    Hold,
    Toggle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InsertMethod {
    Paste,
    Clipboard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LlmProviderKind {
    None,
    Ollama,
    OpenaiCompatible,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmConfig {
    pub provider: LlmProviderKind,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub timeout_secs: u64,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: LlmProviderKind::None,
            base_url: "http://localhost:11434".into(),
            api_key: String::new(),
            model: "qwen2.5:3b".into(),
            timeout_secs: 30,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Mode {
    pub id: String,
    pub name: String,
    pub built_in: bool,
    pub uses_llm: bool,
    pub prompt: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DictionaryEntry {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub version: u32,
    pub dictation_hotkey: String,
    pub dictation_hotkey_behavior: HotkeyBehavior,
    pub refine_hotkey: String,
    pub active_mode_id: String,
    pub modes: Vec<Mode>,
    pub dictionary: Vec<DictionaryEntry>,
    pub stt_model_id: String,
    pub language: String,
    pub llm: LlmConfig,
    pub insert_method: InsertMethod,
    pub restore_clipboard: bool,
    pub launch_at_login: bool,
    pub onboarding_completed: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: SETTINGS_VERSION,
            dictation_hotkey: "Alt+Space".into(),
            dictation_hotkey_behavior: HotkeyBehavior::Hold,
            refine_hotkey: "Alt+Shift+Space".into(),
            active_mode_id: modes::STANDARD_MODE_ID.into(),
            modes: modes::built_in_modes(),
            dictionary: Vec::new(),
            stt_model_id: "base.en".into(),
            language: "auto".into(),
            llm: LlmConfig::default(),
            insert_method: InsertMethod::Paste,
            restore_clipboard: true,
            launch_at_login: false,
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
        for built_in in modes::built_in_modes() {
            if !self.modes.iter().any(|m| m.id == built_in.id) {
                self.modes.push(built_in);
            }
        }
        if !self.modes.iter().any(|m| m.id == self.active_mode_id) {
            self.active_mode_id = modes::STANDARD_MODE_ID.into();
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
        if let Err(err) = manager.persist() {
            log::warn!("could not persist settings on load: {err}");
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
        assert!(json.contains("\"provider\":\"none\""));
        assert!(json.contains("\"insertMethod\":\"paste\""));
    }
}
