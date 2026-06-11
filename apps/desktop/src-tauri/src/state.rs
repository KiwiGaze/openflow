//! Shared application state managed by Tauri and reachable from commands,
//! tray handlers, and shortcut handlers via `app.state::<AppState>()`.

use std::sync::Arc;

use crate::llm::LlmClient;
use crate::models::ModelManager;
use crate::output::OutputSystem;
use crate::pipeline::Pipeline;
use crate::profiles::ProfileManager;
use crate::settings::SettingsManager;
use crate::stt::SttEngine;

/// The audio system is owned by the pipeline; everything commands or tray
/// handlers need lives here.
pub struct AppState {
    pub settings: Arc<SettingsManager>,
    pub profiles: Arc<ProfileManager>,
    pub models: Arc<ModelManager>,
    pub stt: Arc<SttEngine>,
    pub llm: Arc<LlmClient>,
    pub output: Arc<OutputSystem>,
    pub pipeline: Arc<Pipeline>,
}
