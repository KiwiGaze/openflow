//! Opt-in local dictation history. Off by default (`settings.history_enabled`);
//! a privacy posture the user turns on explicitly. One JSON file under
//! `<app-data>`, capped and atomically written, storing **text only — never
//! audio**. Serialized camelCase; the TS mirror is `packages/core/src/types.ts`.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// Bounds the file: oldest entries drop past this. History is a convenience,
/// not an archive.
const MAX_ENTRIES: usize = 200;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: String,
    pub raw: String,
    pub text: String,
    pub mode_id: String,
    pub refined: bool,
    /// Unix epoch milliseconds; the webview formats the date locally.
    pub at: u64,
}

pub struct HistoryStore {
    path: PathBuf,
    entries: RwLock<Vec<HistoryEntry>>,
    seq: AtomicU64,
}

impl HistoryStore {
    pub fn load(dir: &Path) -> Self {
        let path = dir.join("history.json");
        let entries: Vec<HistoryEntry> = match fs::read_to_string(&path) {
            Ok(raw) => match serde_json::from_str(&raw) {
                Ok(entries) => entries,
                Err(err) => {
                    // Don't clobber a corrupt-but-present log: the next append()
                    // would overwrite it. Preserve it so transcripts aren't lost.
                    log::warn!("history.json unreadable ({err}); preserving as .corrupt");
                    if let Err(rename_err) = fs::rename(&path, path.with_extension("json.corrupt"))
                    {
                        // Rename failed → the corrupt file is still in place and
                        // the next append() will overwrite it. Make that loss
                        // visible instead of silent.
                        log::warn!("could not preserve corrupt history.json: {rename_err}");
                    }
                    Vec::new()
                }
            },
            Err(_) => Vec::new(),
        };
        Self {
            path,
            entries: RwLock::new(entries),
            seq: AtomicU64::new(0),
        }
    }

    pub fn list(&self) -> Vec<HistoryEntry> {
        self.entries.read().expect("history lock poisoned").clone()
    }

    /// Append a dictation (newest first), capped. Best-effort persist — a
    /// history write failure must never affect the dictation that just landed.
    pub fn append(&self, raw: String, text: String, mode_id: String, refined: bool) {
        let at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let seq = self.seq.fetch_add(1, Ordering::SeqCst);
        let entry = HistoryEntry {
            id: format!("{at}-{seq}"),
            raw,
            text,
            mode_id,
            refined,
            at,
        };
        {
            let mut guard = self.entries.write().expect("history lock poisoned");
            guard.insert(0, entry);
            guard.truncate(MAX_ENTRIES);
        }
        if let Err(err) = self.persist() {
            log::warn!("could not persist history: {err}");
        }
    }

    /// Clears the log and removes the file. Deletes first and only clears the
    /// in-memory copy on success: a "cleared" history still sitting on disk is a
    /// privacy lie, so the failure must surface to the caller.
    pub fn clear(&self) -> std::io::Result<()> {
        match fs::remove_file(&self.path) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => return Err(err),
        }
        self.entries.write().expect("history lock poisoned").clear();
        Ok(())
    }

    fn persist(&self) -> std::io::Result<()> {
        let entries = self.list();
        let json = serde_json::to_string_pretty(&entries).unwrap_or_else(|_| "[]".into());
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let tmp = self.path.with_extension("json.tmp");
        fs::write(&tmp, json)?;
        fs::rename(&tmp, &self.path)
    }
}
