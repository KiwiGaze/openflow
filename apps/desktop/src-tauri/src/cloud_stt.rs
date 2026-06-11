//! Generic cloud speech-to-text over the OpenAI-audio multipart shape
//! (`/audio/transcriptions` with `file`, `model`, `language`, `prompt`) — the
//! one client covering whisper-server / Faster-Whisper / OpenAI / Groq (08 §2).
//!
//! This is the only path that **uploads audio off the Mac**. It is reached only
//! after the per-profile consent gate is confirmed (see `pipeline.rs`); this
//! module performs the upload but never decides whether it is allowed.

use std::time::Duration;

use crate::error::{AppError, AppResult};
use crate::resample::WHISPER_SAMPLE_RATE;
use crate::stt_profiles::SttProfile;

/// Encodes 16 kHz mono `f32` samples as a 16-bit PCM WAV in-process (08 §8).
fn encode_wav(samples: &[f32]) -> Vec<u8> {
    let sample_rate = WHISPER_SAMPLE_RATE;
    let data_size = (samples.len() * 2) as u32;
    let mut buf = Vec::with_capacity(44 + data_size as usize);
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&(36 + data_size).to_le_bytes());
    buf.extend_from_slice(b"WAVE");
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes()); // PCM fmt chunk size
    buf.extend_from_slice(&1u16.to_le_bytes()); // PCM
    buf.extend_from_slice(&1u16.to_le_bytes()); // mono
    buf.extend_from_slice(&sample_rate.to_le_bytes());
    buf.extend_from_slice(&(sample_rate * 2).to_le_bytes()); // byte rate
    buf.extend_from_slice(&2u16.to_le_bytes()); // block align
    buf.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_size.to_le_bytes());
    for &s in samples {
        let v = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
        buf.extend_from_slice(&v.to_le_bytes());
    }
    buf
}

#[derive(serde::Deserialize)]
struct AudioResponse {
    text: String,
}

/// Uploads the recording and returns the transcript. The base URL already
/// carries its path (`/v1`, `/openai/v1`); we append `/audio/transcriptions`.
pub async fn transcribe(
    profile: &SttProfile,
    samples: &[f32],
    language: &str,
    prompt: Option<&str>,
) -> AppResult<String> {
    let url = format!(
        "{}/audio/transcriptions",
        profile.base_url.trim_end_matches('/')
    );
    let part = reqwest::multipart::Part::bytes(encode_wav(samples))
        .file_name("audio.wav")
        .mime_str("audio/wav")
        .map_err(|e| AppError::Stt(format!("could not build the audio upload: {e}")))?;
    let mut form = reqwest::multipart::Form::new()
        .part("file", part)
        .text("model", profile.model.clone())
        .text("response_format", "json");
    if language != "auto" {
        form = form.text("language", language.to_string());
    }
    if let Some(prompt) = prompt {
        form = form.text("prompt", prompt.to_string());
    }

    let client = reqwest::Client::new();
    let mut req = client
        .post(&url)
        .multipart(form)
        .timeout(Duration::from_secs(profile.timeout_secs.max(5)));
    if !profile.api_key.trim().is_empty() {
        req = req.bearer_auth(profile.api_key.trim());
    }

    let resp = req
        .send()
        .await
        .map_err(|e| AppError::Stt(format!("cloud transcription request failed: {e}")))?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        log::warn!("cloud STT {status}: {body}");
        return Err(AppError::Stt(format!(
            "cloud transcription failed ({status})"
        )));
    }
    let parsed: AudioResponse = resp
        .json()
        .await
        .map_err(|e| AppError::Stt(format!("cloud transcription response was invalid: {e}")))?;
    Ok(parsed.text.trim().to_string())
}
