//! Local speech-to-text via whisper.cpp (through the `whisper-rs` bindings).
//!
//! The model context is expensive to create (hundreds of MB mapped into GPU
//! memory), so it is loaded once and reused until the user switches models.
//! Calls run inside `spawn_blocking`; the mutex serializes inference.

use std::path::Path;
use std::sync::Mutex;

use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

use crate::error::{AppError, AppResult};
use crate::resample::{rms, WHISPER_SAMPLE_RATE};

/// Below this RMS the recording is treated as silence and skipped — whisper
/// hallucinates text on empty audio.
const SILENCE_RMS_THRESHOLD: f32 = 0.000_5;

/// Whisper misbehaves on very short clips; pad with trailing silence.
const MIN_SAMPLES: usize = (WHISPER_SAMPLE_RATE as usize * 11) / 10; // 1.1 s

struct LoadedModel {
    model_id: String,
    context: WhisperContext,
}

#[derive(Default)]
pub struct SttEngine {
    loaded: Mutex<Option<LoadedModel>>,
}

impl SttEngine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Drops the loaded context (e.g. after the model file is deleted).
    pub fn unload(&self) {
        *self.loaded.lock().expect("stt lock poisoned") = None;
    }

    /// Transcribes 16 kHz mono samples. `language` is an ISO 639-1 code or
    /// "auto"; English-only models are always forced to English.
    pub fn transcribe(
        &self,
        model_id: &str,
        model_path: &Path,
        samples: &[f32],
        language: &str,
        initial_prompt: Option<&str>,
    ) -> AppResult<String> {
        if rms(samples) < SILENCE_RMS_THRESHOLD {
            return Ok(String::new());
        }

        let mut padded;
        let samples = if samples.len() < MIN_SAMPLES {
            padded = samples.to_vec();
            padded.resize(MIN_SAMPLES, 0.0);
            &padded[..]
        } else {
            samples
        };

        let mut guard = self.loaded.lock().expect("stt lock poisoned");
        let needs_load = guard
            .as_ref()
            .map(|m| m.model_id != model_id)
            .unwrap_or(true);
        if needs_load {
            *guard = None; // free the old context before mapping the new one
            let path_str = model_path
                .to_str()
                .ok_or_else(|| AppError::Model("model path is not valid UTF-8".into()))?;
            log::info!("loading whisper model {model_id} from {path_str}");
            let context =
                WhisperContext::new_with_params(path_str, WhisperContextParameters::default())
                    .map_err(|e| {
                        AppError::Model(format!("failed to load model {model_id}: {e}"))
                    })?;
            *guard = Some(LoadedModel {
                model_id: model_id.to_string(),
                context,
            });
        }
        let loaded = guard
            .as_ref()
            .ok_or_else(|| AppError::Stt("whisper context missing after load".into()))?;

        let mut state = loaded
            .context
            .create_state()
            .map_err(|e| AppError::Stt(format!("failed to create state: {e}")))?;

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_translate(false);
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_suppress_blank(true);
        params.set_no_context(true);

        let english_only = model_id.ends_with(".en");
        let lang = if english_only {
            Some("en")
        } else if language == "auto" {
            None
        } else {
            Some(language)
        };
        params.set_language(lang);

        if let Some(prompt) = initial_prompt {
            params.set_initial_prompt(prompt);
        }

        state
            .full(params, samples)
            .map_err(|e| AppError::Stt(format!("inference failed: {e}")))?;

        let n_segments = state.full_n_segments();
        let mut text = String::new();
        for i in 0..n_segments {
            if let Some(segment) = state.get_segment(i) {
                text.push_str(&segment.to_string());
            }
        }
        Ok(text.trim().to_string())
    }
}

/// Builds the vocabulary-biasing prompt whisper sees before the audio.
pub fn initial_prompt_from_dictionary(
    entries: &[crate::settings::DictionaryEntry],
) -> Option<String> {
    if entries.is_empty() {
        return None;
    }
    let words: Vec<&str> = entries.iter().map(|e| e.to.as_str()).collect();
    Some(format!("Glossary: {}.", words.join(", ")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::DictionaryEntry;

    #[test]
    fn silence_short_circuits_without_a_model() {
        let engine = SttEngine::new();
        let silence = vec![0.0f32; WHISPER_SAMPLE_RATE as usize];
        let out = engine
            .transcribe("base.en", Path::new("/nonexistent"), &silence, "auto", None)
            .unwrap();
        assert_eq!(out, "");
    }

    #[test]
    fn initial_prompt_lists_dictionary_targets() {
        assert_eq!(initial_prompt_from_dictionary(&[]), None);
        let entries = vec![
            DictionaryEntry {
                from: "open flow".into(),
                to: "OpenFlow".into(),
            },
            DictionaryEntry {
                from: "k 8 s".into(),
                to: "k8s".into(),
            },
        ];
        assert_eq!(
            initial_prompt_from_dictionary(&entries).unwrap(),
            "Glossary: OpenFlow, k8s."
        );
    }

    /// Real-model integration test; runs only when a model file is provided:
    /// `OPENFLOW_TEST_MODEL=~/path/ggml-tiny.en.bin cargo test -- --ignored`
    #[test]
    #[ignore = "requires a downloaded whisper model"]
    fn transcribes_synthetic_audio_with_real_model() {
        let model = std::env::var("OPENFLOW_TEST_MODEL").expect("OPENFLOW_TEST_MODEL not set");
        let engine = SttEngine::new();
        // 2 s of a 220 Hz tone — expect empty or near-empty output, but the
        // full load→infer path must not error.
        let samples: Vec<f32> = (0..WHISPER_SAMPLE_RATE * 2)
            .map(|i| {
                (2.0 * std::f32::consts::PI * 220.0 * i as f32 / WHISPER_SAMPLE_RATE as f32).sin()
                    * 0.05
            })
            .collect();
        let out = engine
            .transcribe("test", Path::new(&model), &samples, "en", None)
            .unwrap();
        assert!(out.len() < 200);
    }
}
