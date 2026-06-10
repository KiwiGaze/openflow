use serde::{Serialize, Serializer};

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("audio error: {0}")]
    Audio(String),

    #[error("transcription error: {0}")]
    Stt(String),

    #[error("model error: {0}")]
    Model(String),

    #[error("AI provider error: {0}")]
    Llm(String),

    #[error("output error: {0}")]
    Output(String),

    #[error("settings error: {0}")]
    Settings(String),

    /// Invalid operation for the current pipeline state (e.g. starting a
    /// recording while one is already running).
    #[error("{0}")]
    State(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("window error: {0}")]
    Tauri(#[from] tauri::Error),
}

/// Tauri commands serialize errors for the frontend; a plain message is all
/// the UI needs.
impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}
