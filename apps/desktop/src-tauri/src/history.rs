//! Opt-in local dictation history. Off by default (`settings.history_enabled`);
//! a privacy posture the user turns on explicitly. Persisted in the shared
//! SQLite store (`db.rs`), storing **text only — never audio**. Newest first,
//! hard-capped. Serialized camelCase; the TS mirror is `packages/core/src/types.ts`.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::db::{Db, HistoryRow};
use crate::error::AppResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: String,
    /// Unix epoch milliseconds; the webview formats the date locally.
    pub at: i64,
    pub text: String,
    pub raw_text: String,
    pub mode_id: String,
    /// Frontmost app's display name at dictation time; null for legacy imports.
    pub app_name: Option<String>,
    /// Recording duration in milliseconds; null for legacy imports.
    pub duration_ms: Option<i64>,
    pub word_count: i64,
    /// Whether an LLM pass ran (vs rules-based cleanup only).
    pub used_ai: bool,
}

impl From<HistoryRow> for HistoryEntry {
    fn from(row: HistoryRow) -> Self {
        Self {
            id: row.id,
            at: row.at,
            text: row.text,
            raw_text: row.raw_text,
            mode_id: row.mode_id,
            app_name: row.app_name,
            duration_ms: row.duration_ms,
            word_count: row.word_count,
            used_ai: row.used_ai,
        }
    }
}

/// Owns history logic over the shared SQLite store. The `seq` counter only
/// disambiguates two appends landing in the same millisecond when minting ids.
pub struct HistoryStore {
    db: Arc<Db>,
    seq: AtomicU64,
}

impl HistoryStore {
    pub fn new(db: Arc<Db>) -> Self {
        Self {
            db,
            seq: AtomicU64::new(0),
        }
    }

    pub fn list(&self) -> Vec<HistoryEntry> {
        match self.db.history_list() {
            Ok(rows) => rows.into_iter().map(HistoryEntry::from).collect(),
            Err(err) => {
                log::warn!("could not read history: {err}");
                Vec::new()
            }
        }
    }

    /// Append a dictation (newest first), capped. Best-effort persist — a
    /// history write failure must never affect the dictation that just landed.
    #[allow(clippy::too_many_arguments)]
    pub fn append(
        &self,
        raw: String,
        text: String,
        mode_id: String,
        app_name: Option<String>,
        duration_ms: Option<i64>,
        word_count: i64,
        used_ai: bool,
    ) {
        let at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        let seq = self.seq.fetch_add(1, Ordering::SeqCst);
        let id = format!("{at}-{seq}");
        if let Err(err) = self.db.history_append(
            &id,
            at,
            &text,
            &raw,
            &mode_id,
            app_name.as_deref(),
            duration_ms,
            word_count,
            used_ai,
        ) {
            log::warn!("could not persist history: {err}");
        }
    }

    /// Removes every history row. A "cleared" history that still has rows is a
    /// privacy lie, so a failure must surface to the caller.
    pub fn clear(&self) -> AppResult<()> {
        self.db.history_clear()
    }

    /// Removes one history row by id.
    pub fn delete(&self, id: &str) -> AppResult<()> {
        self.db.history_delete(id)
    }
}
