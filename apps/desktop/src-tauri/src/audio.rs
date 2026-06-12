//! Microphone capture on a dedicated thread.
//!
//! `cpal::Stream` is not `Send`, so the stream lives on its own OS thread and
//! the rest of the app talks to it through a channel. The capture callback
//! appends to a shared buffer and publishes an RMS level for the HUD meter.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{self, Sender, SyncSender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use crate::error::{AppError, AppResult};
use crate::resample::{downmix_to_mono, resample_to_16k, rms};
use crate::settings::MAX_RECORDING_SECS;

/// Mono samples at 16 kHz, ready for whisper.
pub struct RecordedAudio {
    pub samples: Vec<f32>,
    /// Wall-clock duration of the recording.
    pub duration: Duration,
}

enum AudioCmd {
    Start {
        respond: SyncSender<AppResult<()>>,
    },
    Stop {
        respond: SyncSender<AppResult<RecordedAudio>>,
    },
    Cancel,
}

pub struct AudioSystem {
    tx: Sender<AudioCmd>,
    /// Latest input level (RMS of the most recent callback chunk), stored as
    /// `f32::to_bits` so it can live in an atomic.
    level: Arc<AtomicU32>,
}

impl AudioSystem {
    pub fn spawn() -> Self {
        let (tx, rx) = mpsc::channel::<AudioCmd>();
        let level = Arc::new(AtomicU32::new(0));
        let level_for_thread = Arc::clone(&level);

        std::thread::Builder::new()
            .name("velata-audio".into())
            .spawn(move || worker(rx, level_for_thread))
            .expect("failed to spawn audio thread");

        Self { tx, level }
    }

    pub fn start(&self) -> AppResult<()> {
        let (respond, wait) = mpsc::sync_channel(1);
        self.tx
            .send(AudioCmd::Start { respond })
            .map_err(|_| AppError::Audio("audio thread is gone".into()))?;
        wait.recv()
            .map_err(|_| AppError::Audio("audio thread did not respond".into()))?
    }

    pub fn stop(&self) -> AppResult<RecordedAudio> {
        let (respond, wait) = mpsc::sync_channel(1);
        self.tx
            .send(AudioCmd::Stop { respond })
            .map_err(|_| AppError::Audio("audio thread is gone".into()))?;
        wait.recv()
            .map_err(|_| AppError::Audio("audio thread did not respond".into()))?
    }

    pub fn cancel(&self) {
        let _ = self.tx.send(AudioCmd::Cancel);
    }

    /// Most recent input level in [0, 1]-ish RMS units.
    pub fn level(&self) -> f32 {
        f32::from_bits(self.level.load(Ordering::Relaxed))
    }
}

struct ActiveRecording {
    // Held only to keep the stream alive; dropping it stops capture.
    _stream: cpal::Stream,
    buffer: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
    started: std::time::Instant,
}

fn worker(rx: mpsc::Receiver<AudioCmd>, level: Arc<AtomicU32>) {
    let mut active: Option<ActiveRecording> = None;

    while let Ok(cmd) = rx.recv() {
        match cmd {
            AudioCmd::Start { respond } => {
                let result = if active.is_some() {
                    Err(AppError::State("already recording".into()))
                } else {
                    match open_stream(Arc::clone(&level)) {
                        Ok(recording) => {
                            active = Some(recording);
                            Ok(())
                        }
                        Err(err) => Err(err),
                    }
                };
                let _ = respond.send(result);
            }
            AudioCmd::Stop { respond } => {
                let result = match active.take() {
                    None => Err(AppError::State("not recording".into())),
                    Some(recording) => {
                        let duration = recording.started.elapsed();
                        let sample_rate = recording.sample_rate;
                        let raw = std::mem::take(
                            &mut *recording.buffer.lock().expect("audio buffer poisoned"),
                        );
                        // Drop stops the stream before the (potentially slow)
                        // resample so the mic indicator turns off immediately.
                        drop(recording);
                        level.store(0, Ordering::Relaxed);
                        let samples = resample_to_16k(&raw, sample_rate);
                        Ok(RecordedAudio { samples, duration })
                    }
                };
                let _ = respond.send(result);
            }
            AudioCmd::Cancel => {
                active = None;
                level.store(0, Ordering::Relaxed);
            }
        }
    }
}

fn open_stream(level: Arc<AtomicU32>) -> AppResult<ActiveRecording> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| AppError::Audio("no microphone found".into()))?;
    let config = device
        .default_input_config()
        .map_err(|e| AppError::Audio(format!("no usable microphone config: {e}")))?;

    let sample_rate = config.sample_rate();
    let channels = config.channels();
    let max_samples = (sample_rate as u64 * MAX_RECORDING_SECS) as usize;

    let buffer: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
    let buffer_for_cb = Arc::clone(&buffer);

    let stream = device
        .build_input_stream(
            config.into(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let mono = downmix_to_mono(data, channels);
                level.store(rms(&mono).to_bits(), Ordering::Relaxed);
                let mut buf = buffer_for_cb.lock().expect("audio buffer poisoned");
                if buf.len() < max_samples {
                    buf.extend_from_slice(&mono);
                }
            },
            |err| log::error!("input stream error: {err}"),
            None,
        )
        .map_err(|e| AppError::Audio(format!("could not open microphone: {e}")))?;

    stream
        .play()
        .map_err(|e| AppError::Audio(format!("could not start capture: {e}")))?;

    Ok(ActiveRecording {
        _stream: stream,
        buffer,
        sample_rate,
        started: std::time::Instant::now(),
    })
}
