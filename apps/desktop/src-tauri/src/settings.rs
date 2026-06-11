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
    pub prompt: String,
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
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub version: u32,
    pub dictation_hotkey: String,
    pub dictation_hotkey_behavior: HotkeyBehavior,
    pub refine_hotkey: String,
    pub polish_hotkey: String,
    /// Master switch: may dictation transcripts go to the active profile.
    pub refine_after_dictation: bool,
    /// Active profile id (a file under `<app-data>/profiles/`); "" = no AI.
    pub active_llm_profile_id: String,
    pub active_mode_id: String,
    pub modes: Vec<Mode>,
    pub dictionary: Vec<DictionaryEntry>,
    pub snippets: Vec<Snippet>,
    pub transforms: Vec<Transform>,
    pub stt_model_id: String,
    pub language: String,
    /// v1 migration only — see `profiles::reconcile`.
    #[serde(skip_serializing)]
    pub llm: Option<LegacyLlmConfig>,
    pub insert_method: InsertMethod,
    pub restore_clipboard: bool,
    pub launch_at_login: bool,
    /// Keep a Dock icon (Regular activation). Off = menu-bar-only (Accessory).
    pub show_in_dock: bool,
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
            refine_after_dictation: true,
            active_llm_profile_id: String::new(),
            active_mode_id: modes::STANDARD_MODE_ID.into(),
            modes: modes::built_in_modes(),
            dictionary: Vec::new(),
            snippets: Vec::new(),
            transforms: Vec::new(),
            stt_model_id: "base.en".into(),
            language: "auto".into(),
            llm: None,
            insert_method: InsertMethod::Paste,
            restore_clipboard: true,
            launch_at_login: false,
            show_in_dock: false,
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
        self.snippets
            .retain(|s| !s.trigger.trim().is_empty() && !s.expansion.is_empty());
        // A transform needs a name; an empty instruction is a valid draft (it
        // falls back to the Polish default until the user fills it in).
        self.transforms.retain(|t| !t.name.trim().is_empty());
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
        assert!(json.contains("\"refineAfterDictation\":true"));
        assert!(json.contains("\"activeLlmProfileId\":\"\""));
        assert!(json.contains("\"insertMethod\":\"paste\""));
        assert!(json.contains("\"snippets\":[]"));
        assert!(json.contains("\"showInDock\":false"));
        // The v1 LLM block is deserialize-only; it must never be written.
        assert!(!json.contains("\"llm\""));
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
    fn normalize_drops_blank_name_transforms_but_keeps_instruction_drafts() {
        let dir = temp_dir("transforms");
        let manager = SettingsManager::load(&dir);
        let mut s = manager.get();
        s.transforms = vec![
            Transform {
                id: "a".into(),
                name: "Concise".into(),
                instruction: "Tighten the wording.".into(),
                hotkey: "Alt+1".into(),
            },
            // A named draft with no instruction yet is kept (acts like Polish).
            Transform {
                id: "b".into(),
                name: "Draft".into(),
                instruction: String::new(),
                hotkey: String::new(),
            },
            // No name → dropped.
            Transform {
                id: "c".into(),
                name: "  ".into(),
                instruction: "orphaned".into(),
                hotkey: String::new(),
            },
        ];
        let fixed = manager.set(s).unwrap();
        assert_eq!(fixed.transforms.len(), 2);
        assert!(fixed.transforms.iter().any(|t| t.name == "Draft"));
        assert!(!fixed.transforms.iter().any(|t| t.id == "c"));
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
