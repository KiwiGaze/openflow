//! LLM profiles: one JSON file per profile under `<app-data>/profiles/`.
//!
//! A profile is a named connection for refinement (provider, base URL, key,
//! model). Exactly one profile may be active at a time; the pointer lives in
//! settings (`activeLlmProfileId`, empty = no AI). The files are the source
//! of truth: the filename stem is the profile identity, hand-dropped files
//! appear on the next scan, and unreadable files are skipped, never deleted.
//! Serialized camelCase — the TypeScript mirror lives in
//! `packages/core/src/types.ts`; update both sides together.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::settings::SettingsManager;

pub const PROFILE_VERSION: u32 = 1;
/// Id of the profile created from the v1 inline `llm` settings block.
pub const MIGRATED_PROFILE_ID: &str = "default";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LlmProviderKind {
    Ollama,
    OpenaiCompatible,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LlmProfile {
    pub version: u32,
    /// Identity; always equals the filename stem.
    pub id: String,
    pub name: String,
    pub provider: LlmProviderKind,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub timeout_secs: u64,
}

impl Default for LlmProfile {
    fn default() -> Self {
        Self {
            version: PROFILE_VERSION,
            id: String::new(),
            name: String::new(),
            provider: LlmProviderKind::Ollama,
            base_url: "http://localhost:11434".into(),
            api_key: String::new(),
            model: String::new(),
            timeout_secs: 30,
        }
    }
}

/// Ids double as filename stems; reject anything that could leave the dir.
fn safe_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && !id.contains('/')
        && !id.contains('\\')
        && !id.contains("..")
        && id != "."
}

pub struct ProfileManager {
    dir: PathBuf,
    cache: RwLock<Vec<LlmProfile>>,
}

impl ProfileManager {
    pub fn new(dir: PathBuf) -> Self {
        let cache = scan(&dir);
        Self {
            dir,
            cache: RwLock::new(cache),
        }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Rescans the directory (hand-dropped files show up here) and returns
    /// every readable profile.
    pub fn list(&self) -> Vec<LlmProfile> {
        let fresh = scan(&self.dir);
        *self.cache.write().expect("profile cache poisoned") = fresh.clone();
        fresh
    }

    pub fn get(&self, id: &str) -> Option<LlmProfile> {
        self.cache
            .read()
            .expect("profile cache poisoned")
            .iter()
            .find(|p| p.id == id)
            .cloned()
    }

    /// The profile selected for refinement, or `None` when "No AI" is chosen
    /// or the pointer is dangling.
    pub fn active(&self, active_id: &str) -> Option<LlmProfile> {
        if active_id.is_empty() {
            None
        } else {
            self.get(active_id)
        }
    }

    /// Upserts a profile atomically (temp + rename) with 0600 permissions —
    /// profile files can hold API keys. Returns the fresh list.
    pub fn save(&self, mut profile: LlmProfile) -> AppResult<Vec<LlmProfile>> {
        if !safe_id(&profile.id) {
            return Err(AppError::Settings(format!(
                "invalid profile id “{}”",
                profile.id
            )));
        }
        profile.version = PROFILE_VERSION;
        if profile.name.trim().is_empty() {
            profile.name = "Untitled profile".into();
        }
        fs::create_dir_all(&self.dir)?;
        let json = serde_json::to_string_pretty(&profile)
            .map_err(|e| AppError::Settings(e.to_string()))?;
        let path = self.path_for(&profile.id);
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, json)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&tmp, fs::Permissions::from_mode(0o600))?;
        }
        fs::rename(&tmp, &path)?;
        Ok(self.list())
    }

    /// Deletes the profile file (already gone is fine) and returns the fresh
    /// list. Clearing a dangling active pointer is the caller's job.
    pub fn delete(&self, id: &str) -> AppResult<Vec<LlmProfile>> {
        if !safe_id(id) {
            return Err(AppError::Settings(format!("invalid profile id “{id}”")));
        }
        match fs::remove_file(self.path_for(id)) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => return Err(err.into()),
        }
        Ok(self.list())
    }

    fn path_for(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{id}.json"))
    }
}

fn scan(dir: &Path) -> Vec<LlmProfile> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut profiles: Vec<LlmProfile> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        let raw = match fs::read_to_string(&path) {
            Ok(raw) => raw,
            Err(err) => {
                log::warn!("skipping unreadable profile {}: {err}", path.display());
                continue;
            }
        };
        match serde_json::from_str::<LlmProfile>(&raw) {
            Ok(mut profile) => {
                // The filename is the identity; covers hand-copied files.
                profile.id = stem.to_string();
                profiles.push(profile);
            }
            Err(err) => log::warn!("skipping corrupt profile {}: {err}", path.display()),
        }
    }
    profiles.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.cmp(&b.id)));
    profiles
}

/// One-time bridge from v1 settings plus pointer repair. Moves the inline
/// `llm` block into a profile file and points `activeLlmProfileId` at it,
/// then clears the pointer if the active profile's file is gone. Safe to call
/// on every launch.
pub fn reconcile(settings: &SettingsManager, profiles: &ProfileManager) {
    let current = settings.get();
    let mut next = current.clone();
    let mut dirty = false;

    if let Some(legacy) = current.llm {
        next.llm = None;
        dirty = true;
        let provider = match legacy.provider.as_str() {
            "ollama" => Some(LlmProviderKind::Ollama),
            "openaiCompatible" => Some(LlmProviderKind::OpenaiCompatible),
            // "none" or unknown → no profile to create.
            _ => None,
        };
        if let Some(provider) = provider {
            let label = match provider {
                LlmProviderKind::Ollama => "Ollama",
                LlmProviderKind::OpenaiCompatible => "OpenAI-compatible",
            };
            let name = if legacy.model.trim().is_empty() {
                label.to_string()
            } else {
                format!("{label} — {}", legacy.model)
            };
            let profile = LlmProfile {
                version: PROFILE_VERSION,
                id: MIGRATED_PROFILE_ID.into(),
                name,
                provider,
                base_url: legacy.base_url,
                api_key: legacy.api_key,
                model: legacy.model,
                timeout_secs: if legacy.timeout_secs == 0 {
                    30
                } else {
                    legacy.timeout_secs
                },
            };
            match profiles.save(profile) {
                Ok(_) => next.active_llm_profile_id = MIGRATED_PROFILE_ID.into(),
                Err(err) => {
                    // Keep the legacy block on disk; we retry next launch.
                    log::warn!("could not migrate the v1 LLM config into a profile: {err}");
                    return;
                }
            }
        }
        log::info!("migrated v1 LLM settings into the profiles directory");
    }

    if !next.active_llm_profile_id.is_empty() && profiles.get(&next.active_llm_profile_id).is_none()
    {
        log::warn!(
            "active LLM profile “{}” is missing; refinement is off until one is selected",
            next.active_llm_profile_id
        );
        next.active_llm_profile_id.clear();
        dirty = true;
    }

    if dirty {
        if let Err(err) = settings.set(next) {
            log::warn!("could not persist settings after profile migration: {err}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn sample(id: &str) -> LlmProfile {
        LlmProfile {
            id: id.into(),
            name: format!("Profile {id}"),
            model: "qwen2.5:3b".into(),
            ..LlmProfile::default()
        }
    }

    #[test]
    fn save_list_get_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let manager = ProfileManager::new(dir.path().join("profiles"));
        let listed = manager.save(sample("a")).unwrap();
        assert_eq!(listed.len(), 1);
        let got = manager.get("a").unwrap();
        assert_eq!(got.name, "Profile a");
        assert_eq!(got.version, PROFILE_VERSION);
        assert!(manager.dir().join("a.json").exists());
    }

    #[test]
    fn filename_stem_is_the_identity() {
        let dir = tempfile::tempdir().unwrap();
        let profiles_dir = dir.path().join("profiles");
        fs::create_dir_all(&profiles_dir).unwrap();
        let mut copied = sample("original");
        copied.name = "Copied by hand".into();
        let json = serde_json::to_string(&copied).unwrap();
        fs::write(profiles_dir.join("copy.json"), json).unwrap();

        let manager = ProfileManager::new(profiles_dir);
        let got = manager.get("copy").unwrap();
        assert_eq!(got.id, "copy");
        assert_eq!(got.name, "Copied by hand");
    }

    #[test]
    fn corrupt_files_are_skipped_not_deleted() {
        let dir = tempfile::tempdir().unwrap();
        let profiles_dir = dir.path().join("profiles");
        fs::create_dir_all(&profiles_dir).unwrap();
        fs::write(profiles_dir.join("broken.json"), "{ not json").unwrap();

        let manager = ProfileManager::new(profiles_dir.clone());
        assert!(manager.list().is_empty());
        assert!(profiles_dir.join("broken.json").exists());
    }

    #[test]
    fn delete_removes_the_file_and_tolerates_missing() {
        let dir = tempfile::tempdir().unwrap();
        let manager = ProfileManager::new(dir.path().join("profiles"));
        manager.save(sample("a")).unwrap();
        let listed = manager.delete("a").unwrap();
        assert!(listed.is_empty());
        assert!(!manager.dir().join("a.json").exists());
        assert!(manager.delete("a").unwrap().is_empty());
    }

    #[test]
    fn rejects_ids_that_could_escape_the_directory() {
        let dir = tempfile::tempdir().unwrap();
        let manager = ProfileManager::new(dir.path().join("profiles"));
        for id in ["", ".", "..", "../evil", "a/b", "a\\b"] {
            assert!(manager.save(sample(id)).is_err(), "id {id:?} was accepted");
            assert!(manager.delete(id).is_err(), "id {id:?} was accepted");
        }
    }

    #[test]
    fn active_resolves_only_existing_non_empty_ids() {
        let dir = tempfile::tempdir().unwrap();
        let manager = ProfileManager::new(dir.path().join("profiles"));
        manager.save(sample("a")).unwrap();
        assert!(manager.active("").is_none());
        assert!(manager.active("missing").is_none());
        assert_eq!(manager.active("a").unwrap().id, "a");
    }

    #[cfg(unix)]
    #[test]
    fn profile_files_are_private() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let manager = ProfileManager::new(dir.path().join("profiles"));
        manager.save(sample("a")).unwrap();
        let mode = fs::metadata(manager.dir().join("a.json"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    #[test]
    fn reconcile_migrates_a_v1_llm_block() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("settings.json"),
            r#"{
                "version": 1,
                "llm": {
                    "provider": "ollama",
                    "baseUrl": "http://localhost:11434",
                    "apiKey": "",
                    "model": "qwen2.5:3b",
                    "timeoutSecs": 45
                }
            }"#,
        )
        .unwrap();
        let settings = SettingsManager::load(dir.path());
        assert!(settings.get().llm.is_some());
        let profiles = ProfileManager::new(dir.path().join("profiles"));

        reconcile(&settings, &profiles);

        let migrated = profiles.get(MIGRATED_PROFILE_ID).unwrap();
        assert_eq!(migrated.provider, LlmProviderKind::Ollama);
        assert_eq!(migrated.model, "qwen2.5:3b");
        assert_eq!(migrated.timeout_secs, 45);
        assert_eq!(migrated.name, "Ollama — qwen2.5:3b");
        let s = settings.get();
        assert_eq!(s.active_llm_profile_id, MIGRATED_PROFILE_ID);
        assert!(s.llm.is_none());
        let raw = fs::read_to_string(settings.path()).unwrap();
        assert!(!raw.contains("\"llm\""));

        // Idempotent: a second run changes nothing.
        reconcile(&settings, &profiles);
        assert_eq!(profiles.list().len(), 1);
        assert_eq!(settings.get().active_llm_profile_id, MIGRATED_PROFILE_ID);
    }

    #[test]
    fn reconcile_drops_a_none_provider_without_creating_a_profile() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("settings.json"),
            r#"{ "version": 1, "llm": { "provider": "none" } }"#,
        )
        .unwrap();
        let settings = SettingsManager::load(dir.path());
        let profiles = ProfileManager::new(dir.path().join("profiles"));

        reconcile(&settings, &profiles);

        assert!(profiles.list().is_empty());
        let s = settings.get();
        assert!(s.llm.is_none());
        assert!(s.active_llm_profile_id.is_empty());
    }

    #[test]
    fn reconcile_clears_a_dangling_active_pointer() {
        let dir = tempfile::tempdir().unwrap();
        let settings = SettingsManager::load(dir.path());
        let mut s = settings.get();
        s.active_llm_profile_id = "gone".into();
        settings.set(s).unwrap();
        let profiles = ProfileManager::new(dir.path().join("profiles"));

        reconcile(&settings, &profiles);

        assert!(settings.get().active_llm_profile_id.is_empty());
    }
}
