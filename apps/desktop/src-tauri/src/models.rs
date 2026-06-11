//! Whisper model registry and downloader.
//!
//! Models are ggml files from the official whisper.cpp collection on Hugging
//! Face. They are downloaded on demand into `<app-data>/models/` — never
//! bundled, never uploaded anywhere.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use futures_util::StreamExt;
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::error::{AppError, AppResult};

const HF_BASE: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main";

pub struct ModelSpec {
    pub id: &'static str,
    pub file_name: &'static str,
    pub display_name: &'static str,
    pub size_bytes: u64,
    pub multilingual: bool,
    pub description: &'static str,
}

/// The starter on-device model: `Settings::default()` begins here and a
/// deleted active cloud engine falls back here. The onboarding webview
/// mirrors this id as `DEFAULT_MODEL`.
pub const DEFAULT_STT_MODEL_ID: &str = "base.en";

/// Approximate sizes are shown in the UI; the download trusts Content-Length.
pub const REGISTRY: &[ModelSpec] = &[
    ModelSpec {
        id: "tiny.en",
        file_name: "ggml-tiny.en.bin",
        display_name: "Tiny (English)",
        size_bytes: 77_700_000,
        multilingual: false,
        description: "Fastest, lowest accuracy. Good for quick tests.",
    },
    ModelSpec {
        id: "base.en",
        file_name: "ggml-base.en.bin",
        display_name: "Base (English)",
        size_bytes: 148_000_000,
        multilingual: false,
        description: "Fast with decent accuracy. Good starting point.",
    },
    ModelSpec {
        id: "base",
        file_name: "ggml-base.bin",
        display_name: "Base (Multilingual)",
        size_bytes: 148_000_000,
        multilingual: true,
        description: "Fast, supports ~100 languages.",
    },
    ModelSpec {
        id: "small.en",
        file_name: "ggml-small.en.bin",
        display_name: "Small (English)",
        size_bytes: 488_000_000,
        multilingual: false,
        description: "Balanced speed and accuracy for English.",
    },
    ModelSpec {
        id: "small",
        file_name: "ggml-small.bin",
        display_name: "Small (Multilingual)",
        size_bytes: 488_000_000,
        multilingual: true,
        description: "Balanced speed and accuracy, ~100 languages.",
    },
    ModelSpec {
        id: "large-v3-turbo-q5_0",
        file_name: "ggml-large-v3-turbo-q5_0.bin",
        display_name: "Large v3 Turbo (Quantized)",
        size_bytes: 574_000_000,
        multilingual: true,
        description: "Best quality per MB. Recommended on Apple Silicon.",
    },
    ModelSpec {
        id: "large-v3-turbo",
        file_name: "ggml-large-v3-turbo.bin",
        display_name: "Large v3 Turbo",
        size_bytes: 1_620_000_000,
        multilingual: true,
        description: "Highest quality, largest download.",
    },
];

pub fn spec(id: &str) -> AppResult<&'static ModelSpec> {
    REGISTRY
        .iter()
        .find(|m| m.id == id)
        .ok_or_else(|| AppError::Model(format!("unknown model id: {id}")))
}

pub fn download_url(spec: &ModelSpec) -> String {
    format!("{HF_BASE}/{}", spec.file_name)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfoDto {
    pub id: &'static str,
    pub display_name: &'static str,
    pub file_name: &'static str,
    pub url: String,
    pub size_bytes: u64,
    pub multilingual: bool,
    pub description: &'static str,
    pub installed: bool,
    pub downloading: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub model_id: String,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub done: bool,
    pub error: Option<String>,
}

pub const MODEL_DOWNLOAD_EVENT: &str = "model-download";

pub struct ModelManager {
    models_dir: PathBuf,
    http: reqwest::Client,
    /// Cancel flags for in-flight downloads, keyed by model id.
    active: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

impl ModelManager {
    pub fn new(models_dir: PathBuf) -> Self {
        Self {
            models_dir,
            http: reqwest::Client::new(),
            active: Mutex::new(HashMap::new()),
        }
    }

    pub fn models_dir(&self) -> &Path {
        &self.models_dir
    }

    pub fn path_for(&self, id: &str) -> AppResult<PathBuf> {
        Ok(self.models_dir.join(spec(id)?.file_name))
    }

    pub fn is_installed(&self, id: &str) -> bool {
        spec(id)
            .ok()
            .map(|s| self.models_dir.join(s.file_name).is_file())
            .unwrap_or(false)
    }

    pub fn list(&self) -> Vec<ModelInfoDto> {
        let active = self.active.lock().expect("model lock poisoned");
        REGISTRY
            .iter()
            .map(|s| ModelInfoDto {
                id: s.id,
                display_name: s.display_name,
                file_name: s.file_name,
                url: download_url(s),
                size_bytes: s.size_bytes,
                multilingual: s.multilingual,
                description: s.description,
                installed: self.models_dir.join(s.file_name).is_file(),
                downloading: active.contains_key(s.id),
            })
            .collect()
    }

    pub fn delete(&self, id: &str) -> AppResult<()> {
        let path = self.path_for(id)?;
        if path.is_file() {
            std::fs::remove_file(&path)?;
        }
        Ok(())
    }

    pub fn cancel(&self, id: &str) {
        if let Some(flag) = self.active.lock().expect("model lock poisoned").get(id) {
            flag.store(true, Ordering::Relaxed);
        }
    }

    /// Streams the model to `<file>.part`, emitting progress events, then
    /// renames it into place so an interrupted download is never mistaken for
    /// an installed model.
    pub async fn download(&self, app: &AppHandle, id: &str) -> AppResult<()> {
        let spec = spec(id)?;
        let target = self.models_dir.join(spec.file_name);
        if target.is_file() {
            return Ok(());
        }

        let cancel = Arc::new(AtomicBool::new(false));
        {
            let mut active = self.active.lock().expect("model lock poisoned");
            if active.contains_key(id) {
                return Err(AppError::Model(format!("{id} is already downloading")));
            }
            active.insert(id.to_string(), Arc::clone(&cancel));
        }

        let result = self.download_inner(app, spec, &target, &cancel).await;
        self.active.lock().expect("model lock poisoned").remove(id);

        let progress = match &result {
            Ok(()) => DownloadProgress {
                model_id: id.into(),
                downloaded_bytes: spec.size_bytes,
                total_bytes: spec.size_bytes,
                done: true,
                error: None,
            },
            Err(err) => DownloadProgress {
                model_id: id.into(),
                downloaded_bytes: 0,
                total_bytes: spec.size_bytes,
                done: true,
                error: Some(err.to_string()),
            },
        };
        let _ = app.emit(MODEL_DOWNLOAD_EVENT, &progress);
        result
    }

    async fn download_inner(
        &self,
        app: &AppHandle,
        spec: &ModelSpec,
        target: &Path,
        cancel: &AtomicBool,
    ) -> AppResult<()> {
        tokio::fs::create_dir_all(&self.models_dir).await?;
        let part = target.with_extension("bin.part");

        let response = self
            .http
            .get(download_url(spec))
            .send()
            .await
            .map_err(|e| AppError::Model(format!("download failed to start: {e}")))?
            .error_for_status()
            .map_err(|e| AppError::Model(format!("download rejected: {e}")))?;

        let total = response.content_length().unwrap_or(spec.size_bytes);
        let mut file = tokio::fs::File::create(&part).await?;
        let mut stream = response.bytes_stream();
        let mut downloaded: u64 = 0;
        let mut last_emit = std::time::Instant::now();

        use tokio::io::AsyncWriteExt;
        while let Some(chunk) = stream.next().await {
            if cancel.load(Ordering::Relaxed) {
                drop(file);
                let _ = tokio::fs::remove_file(&part).await;
                return Err(AppError::Model("download cancelled".into()));
            }
            let chunk = chunk.map_err(|e| AppError::Model(format!("download interrupted: {e}")))?;
            file.write_all(&chunk).await?;
            downloaded += chunk.len() as u64;

            if last_emit.elapsed().as_millis() >= 250 {
                last_emit = std::time::Instant::now();
                let _ = app.emit(
                    MODEL_DOWNLOAD_EVENT,
                    &DownloadProgress {
                        model_id: spec.id.into(),
                        downloaded_bytes: downloaded,
                        total_bytes: total,
                        done: false,
                        error: None,
                    },
                );
            }
        }
        file.flush().await?;
        drop(file);

        if total > 0 && downloaded < total {
            let _ = tokio::fs::remove_file(&part).await;
            return Err(AppError::Model(format!(
                "download incomplete ({downloaded} of {total} bytes)"
            )));
        }

        tokio::fs::rename(&part, target).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_ids_and_files_are_unique() {
        let mut ids: Vec<_> = REGISTRY.iter().map(|m| m.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), REGISTRY.len());

        let mut files: Vec<_> = REGISTRY.iter().map(|m| m.file_name).collect();
        files.sort_unstable();
        files.dedup();
        assert_eq!(files.len(), REGISTRY.len());
    }

    #[test]
    fn urls_are_https_hugging_face_resolve_links() {
        for spec in REGISTRY {
            let url = download_url(spec);
            assert!(
                url.starts_with("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-"),
                "unexpected url: {url}"
            );
            assert!(url.ends_with(".bin"));
        }
    }

    #[test]
    fn default_model_exists_in_registry() {
        assert!(spec("base.en").is_ok());
        assert!(spec("nope").is_err());
    }

    #[test]
    fn installed_reflects_file_presence() {
        let dir = std::env::temp_dir().join(format!("openflow-models-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let manager = ModelManager::new(dir.clone());
        assert!(!manager.is_installed("tiny.en"));
        std::fs::write(dir.join("ggml-tiny.en.bin"), b"stub").unwrap();
        assert!(manager.is_installed("tiny.en"));
        assert!(manager
            .list()
            .iter()
            .any(|m| m.id == "tiny.en" && m.installed));
    }
}
