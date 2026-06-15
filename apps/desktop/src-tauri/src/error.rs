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

    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("window error: {0}")]
    Tauri(#[from] tauri::Error),
}

impl AppError {
    /// The bare message without the internal category prefix, for HUD display.
    /// `Display` (and therefore logs) keep the `"x error:"` prefix for triage;
    /// the user only ever sees the message itself.
    pub fn user_message(&self) -> String {
        match self {
            AppError::Audio(m)
            | AppError::Stt(m)
            | AppError::Model(m)
            | AppError::Llm(m)
            | AppError::Output(m)
            | AppError::Settings(m)
            | AppError::State(m) => m.clone(),
            AppError::Io(e) => e.to_string(),
            AppError::Database(e) => e.to_string(),
            AppError::Tauri(e) => e.to_string(),
        }
    }
}

/// Tauri commands serialize errors for the frontend; a plain message is all
/// the UI needs.
impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}
