//! STT engine connections: one JSON file per profile under
//! `<app-data>/stt-profiles/` (08 §2.3). A sibling of the LLM profiles store
//! with the same file-backed pattern — filename stem = identity, 0600, atomic
//! write, corrupt files skipped never deleted. Profiles exist only for engines
//! that need a URL + key; the local whisper.cpp default needs none. Serialized
//! camelCase; the TS mirror is `packages/core/src/types.ts`.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

pub const STT_PROFILE_VERSION: u32 = 1;

/// `Settings::stt_model_id` prefix marking a cloud engine: `cloud:<profile-id>`.
/// Anything else is an on-device whisper model id. Mirrored as
/// `CLOUD_STT_PREFIX` in `@velata/core` — update both together.
pub const CLOUD_STT_PREFIX: &str = "cloud:";

/// Which client transcribes. Only `openaiAudio` (the generic multipart client
/// covering whisper-server / Faster-Whisper / OpenAI / Groq) ships now;
/// Deepgram/AssemblyAI are P3 bespoke clients (08 §7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SttEngineKind {
    #[default]
    OpenaiAudio,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SttProfile {
    pub version: u32,
    /// Identity; always equals the filename stem.
    pub id: String,
    pub name: String,
    pub engine: SttEngineKind,
    /// Display/prefill only (08 §1 mirror); never changes request behavior.
    pub preset_id: String,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub timeout_secs: u64,
}

impl Default for SttProfile {
    fn default() -> Self {
        Self {
            version: STT_PROFILE_VERSION,
            id: String::new(),
            name: String::new(),
            engine: SttEngineKind::OpenaiAudio,
            preset_id: String::new(),
            base_url: String::new(),
            api_key: String::new(),
            model: String::new(),
            timeout_secs: 30,
        }
    }
}

/// Ids double as filename stems; reject anything that could leave the dir.
/// Keep in sync with `profiles::safe_id`.
fn safe_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && !id.contains('/')
        && !id.contains('\\')
        && !id.contains('\0')
        && !id.contains("..")
        && id != "."
}

pub struct SttProfileManager {
    dir: PathBuf,
    cache: RwLock<Vec<SttProfile>>,
}

impl SttProfileManager {
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

    pub fn list(&self) -> Vec<SttProfile> {
        let fresh = scan(&self.dir);
        *self.cache.write().expect("stt profile cache poisoned") = fresh.clone();
        fresh
    }

    pub fn get(&self, id: &str) -> Option<SttProfile> {
        self.cache
            .read()
            .expect("stt profile cache poisoned")
            .iter()
            .find(|p| p.id == id)
            .cloned()
    }

    /// Upserts a profile atomically with 0600 permissions (it can hold an API
    /// key). Returns the fresh list.
    pub fn save(&self, mut profile: SttProfile) -> AppResult<Vec<SttProfile>> {
        if !safe_id(&profile.id) {
            return Err(AppError::Settings(format!(
                "invalid STT profile id “{}”",
                profile.id
            )));
        }
        profile.version = STT_PROFILE_VERSION;
        if profile.name.trim().is_empty() {
            profile.name = "Untitled engine".into();
        }
        fs::create_dir_all(&self.dir)?;
        let json = serde_json::to_string_pretty(&profile)
            .map_err(|e| AppError::Settings(e.to_string()))?;
        let path = self.path_for(&profile.id);
        let tmp = path.with_extension("json.tmp");
        {
            use std::io::Write;
            let mut opts = fs::OpenOptions::new();
            opts.write(true).create(true).truncate(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                // The key must never sit on disk umask-wide, even between
                // create and a later chmod.
                opts.mode(0o600);
            }
            opts.open(&tmp)?.write_all(json.as_bytes())?;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            // mode() applies only on create; a tmp left by a crash keeps its
            // old permissions through the truncating open.
            fs::set_permissions(&tmp, fs::Permissions::from_mode(0o600))?;
        }
        fs::rename(&tmp, &path)?;
        Ok(self.list())
    }

    pub fn delete(&self, id: &str) -> AppResult<Vec<SttProfile>> {
        if !safe_id(id) {
            return Err(AppError::Settings(format!("invalid STT profile id “{id}”")));
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

fn scan(dir: &Path) -> Vec<SttProfile> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut profiles: Vec<SttProfile> = Vec::new();
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
                log::warn!("skipping unreadable STT profile {}: {err}", path.display());
                continue;
            }
        };
        match serde_json::from_str::<SttProfile>(&raw) {
            Ok(mut profile) => {
                profile.id = stem.to_string();
                profiles.push(profile);
            }
            Err(err) => log::warn!("skipping corrupt STT profile {}: {err}", path.display()),
        }
    }
    profiles.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.cmp(&b.id)));
    profiles
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(id: &str) -> SttProfile {
        SttProfile {
            id: id.to_string(),
            name: "Test engine".into(),
            ..SttProfile::default()
        }
    }

    #[test]
    fn rejects_ids_that_could_escape_the_directory() {
        let dir = tempfile::tempdir().unwrap();
        let manager = SttProfileManager::new(dir.path().join("stt-profiles"));
        for id in ["", ".", "..", "../evil", "a/b", "a\\b", "a\0b"] {
            assert!(manager.save(sample(id)).is_err(), "id {id:?} was accepted");
            assert!(manager.delete(id).is_err(), "id {id:?} was accepted");
        }
    }
}
