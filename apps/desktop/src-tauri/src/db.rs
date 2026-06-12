//! The on-device SQLite store. One file (`<app-data>/velata.db`, WAL) holding
//! durable, structured data: dictation history today; insights and notes in
//! later tasks. Text only — **never audio** (that invariant is absolute).
//!
//! Schema is versioned through `PRAGMA user_version` and migrated forward at
//! startup; the runner is idempotent so launching the app twice is a no-op.
//! Migration v1 also imports the legacy `history.json` once (gated by
//! `user_version`), then leaves the file in place — user data is never deleted.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::Connection;
use serde::Deserialize;

use crate::error::{AppError, AppResult};

/// Highest schema version this build knows how to run. Each step is applied in
/// order; the runner stops here.
const SCHEMA_VERSION: i64 = 1;

/// Owns the single SQLite connection behind a `Mutex` (rusqlite's `Connection`
/// is not `Sync`). Cheap to clone-share via `Arc<Db>`.
pub struct Db {
    conn: Mutex<Connection>,
}

impl Db {
    /// Opens (or creates) `<dir>/velata.db`, runs migrations, and returns the
    /// store. A corrupt or unopenable file is renamed aside to
    /// `velata.db.corrupt-<unix-ts>` (never deleted) and a fresh DB is created
    /// in its place.
    pub fn open(dir: &Path) -> AppResult<Self> {
        fs::create_dir_all(dir)?;
        // SQLite creates the -wal/-shm sidecars under the default umask, and
        // the WAL holds history text. Locking the directory itself covers the
        // sidecars (and whatever app-data grows later); the main DB file stays
        // 0600 on top of this.
        fs::set_permissions(dir, fs::Permissions::from_mode(0o700))?;
        let path = dir.join("velata.db");

        let conn = match Self::open_and_migrate(&path) {
            Ok(conn) => conn,
            Err(err) => {
                let aside = corrupt_path(&path);
                log::warn!(
                    "velata.db unusable ({err}); preserving as {} and starting fresh",
                    aside.display()
                );
                // Preserve the user's bytes for recovery; only proceed once the
                // bad file is out of the way so the fresh open can't reopen it.
                fs::rename(&path, &aside)?;
                Self::open_and_migrate(&path)?
            }
        };

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    fn open_and_migrate(path: &Path) -> AppResult<Connection> {
        let existed = path.exists();
        let conn = Connection::open(path)?;
        if !existed {
            // 0600 before any rows land: history is text the user chose to keep
            // private, not world-readable.
            fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
        }
        // WAL survives a crash mid-write and lets reads run during a write.
        conn.pragma_update(None, "journal_mode", "WAL")?;
        migrate(&conn, path)?;
        Ok(conn)
    }

    /// Records one dictation. `app_name`/`duration_ms` are nullable because the
    /// legacy import has neither. Caller passes the final word count (computed
    /// once in the pipeline) rather than recomputing here.
    #[allow(clippy::too_many_arguments)]
    pub fn history_append(
        &self,
        id: &str,
        at: i64,
        text: &str,
        raw_text: &str,
        mode_id: &str,
        app_name: Option<&str>,
        duration_ms: Option<i64>,
        word_count: i64,
        used_ai: bool,
    ) -> AppResult<()> {
        let conn = self.conn.lock().expect("db lock poisoned");
        conn.execute(
            "INSERT INTO history \
             (id, at, text, raw_text, mode_id, app_name, duration_ms, word_count, used_ai) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                id,
                at,
                text,
                raw_text,
                mode_id,
                app_name,
                duration_ms,
                word_count,
                used_ai as i64,
            ],
        )?;
        // Bound the table: a history is a convenience, not an archive.
        conn.execute(
            "DELETE FROM history WHERE id NOT IN \
             (SELECT id FROM history ORDER BY at DESC LIMIT ?1)",
            [HISTORY_CAP],
        )?;
        Ok(())
    }

    /// All history rows, newest first.
    pub fn history_list(&self) -> AppResult<Vec<HistoryRow>> {
        let conn = self.conn.lock().expect("db lock poisoned");
        let mut stmt = conn.prepare(
            "SELECT id, at, text, raw_text, mode_id, app_name, duration_ms, word_count, used_ai \
             FROM history ORDER BY at DESC",
        )?;
        let rows = stmt
            .query_map([], |row| {
                Ok(HistoryRow {
                    id: row.get(0)?,
                    at: row.get(1)?,
                    text: row.get(2)?,
                    raw_text: row.get(3)?,
                    mode_id: row.get(4)?,
                    app_name: row.get(5)?,
                    duration_ms: row.get(6)?,
                    word_count: row.get(7)?,
                    used_ai: row.get::<_, i64>(8)? != 0,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Deletes a single history row. A missing id is not an error.
    pub fn history_delete(&self, id: &str) -> AppResult<()> {
        let conn = self.conn.lock().expect("db lock poisoned");
        conn.execute("DELETE FROM history WHERE id = ?1", [id])?;
        Ok(())
    }

    /// Removes every history row.
    pub fn history_clear(&self) -> AppResult<()> {
        let conn = self.conn.lock().expect("db lock poisoned");
        conn.execute("DELETE FROM history", [])?;
        Ok(())
    }
}

/// One history row as stored. The IPC-facing `HistoryEntry` (history.rs) is a
/// thin rename of this; this struct stays storage-shaped.
#[derive(Debug, Clone)]
pub struct HistoryRow {
    pub id: String,
    pub at: i64,
    pub text: String,
    pub raw_text: String,
    pub mode_id: String,
    pub app_name: Option<String>,
    pub duration_ms: Option<i64>,
    pub word_count: i64,
    pub used_ai: bool,
}

/// Newest-first cap on the history table. Oldest rows drop past this on append.
const HISTORY_CAP: i64 = 1000;

fn corrupt_path(path: &Path) -> PathBuf {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    path.with_extension(format!("db.corrupt-{ts}"))
}

/// Applies pending schema steps in order. Idempotent: a DB already at
/// `SCHEMA_VERSION` does nothing. Each step and its `user_version` bump commit
/// as one transaction, so a crash mid-step rolls back whole — the next launch
/// re-runs the step from the prior version and can never see half-applied
/// schema with a stale version number.
fn migrate(conn: &Connection, db_path: &Path) -> AppResult<()> {
    let mut version: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    while version < SCHEMA_VERSION {
        let tx = conn.unchecked_transaction()?;
        match version {
            0 => migrate_v1(&tx, db_path)?,
            other => {
                return Err(AppError::Settings(format!(
                    "unknown database schema version {other}"
                )))
            }
        }
        version += 1;
        tx.pragma_update(None, "user_version", version)?;
        tx.commit()?;
    }
    Ok(())
}

fn migrate_v1(conn: &Connection, db_path: &Path) -> AppResult<()> {
    conn.execute_batch(
        "CREATE TABLE history (
            id TEXT PRIMARY KEY,
            at INTEGER,
            text TEXT,
            raw_text TEXT,
            mode_id TEXT,
            app_name TEXT,
            duration_ms INTEGER,
            word_count INTEGER,
            used_ai INTEGER
         );
         CREATE TABLE notes (
            id TEXT PRIMARY KEY,
            title TEXT,
            content TEXT,
            created_at INTEGER,
            updated_at INTEGER,
            pinned INTEGER DEFAULT 0,
            deleted_at INTEGER
         );
         CREATE TABLE note_versions (
            id TEXT PRIMARY KEY,
            note_id TEXT REFERENCES notes(id),
            content TEXT,
            source TEXT,
            transform_id TEXT,
            created_at INTEGER
         );
         CREATE TABLE insights_daily (
            day TEXT PRIMARY KEY,
            words INTEGER,
            dictations INTEGER,
            ai_dictations INTEGER,
            fixes INTEGER,
            duration_ms INTEGER
         );
         CREATE INDEX idx_history_at ON history(at);
         CREATE INDEX idx_notes_updated_at ON notes(updated_at);
         CREATE INDEX idx_note_versions_note_id ON note_versions(note_id);",
    )?;
    import_legacy_history(conn, db_path)?;
    Ok(())
}

/// One legacy `history.json` entry (the pre-SQLite shape). Newest first, camelCase.
#[derive(Deserialize)]
struct LegacyHistoryEntry {
    id: String,
    raw: String,
    text: String,
    #[serde(rename = "modeId")]
    mode_id: String,
    polished: bool,
    at: i64,
}

/// Imports `<app-data>/history.json` into the `history` table if present. Runs
/// once — `migrate()` gates it on `user_version`. A corrupt or unreadable file
/// is skipped (never fails the migration, never deleted): worst case the user
/// keeps the JSON and starts a fresh DB history.
fn import_legacy_history(conn: &Connection, db_path: &Path) -> AppResult<()> {
    let Some(dir) = db_path.parent() else {
        return Ok(());
    };
    let json_path = dir.join("history.json");
    let raw = match fs::read_to_string(&json_path) {
        Ok(raw) => raw,
        Err(_) => return Ok(()),
    };
    let entries: Vec<LegacyHistoryEntry> = match serde_json::from_str(&raw) {
        Ok(entries) => entries,
        Err(err) => {
            log::warn!("legacy history.json unreadable ({err}); skipping import, leaving file");
            return Ok(());
        }
    };
    for entry in &entries {
        let word_count = entry.text.split_whitespace().count() as i64;
        conn.execute(
            "INSERT OR IGNORE INTO history \
             (id, at, text, raw_text, mode_id, app_name, duration_ms, word_count, used_ai) \
             VALUES (?1, ?2, ?3, ?4, ?5, NULL, NULL, ?6, ?7)",
            rusqlite::params![
                entry.id,
                entry.at,
                entry.text,
                entry.raw,
                entry.mode_id,
                word_count,
                entry.polished as i64,
            ],
        )?;
    }
    log::info!("imported {} legacy history entries", entries.len());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir() -> tempfile::TempDir {
        tempfile::tempdir().expect("create temp dir")
    }

    fn user_version(db: &Db) -> i64 {
        db.conn
            .lock()
            .expect("db lock poisoned")
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .expect("read user_version")
    }

    #[test]
    fn migrations_are_idempotent() {
        let dir = temp_dir();
        let db = Db::open(dir.path()).expect("first open");
        assert_eq!(user_version(&db), SCHEMA_VERSION);
        drop(db);
        // Re-opening must not re-run migrations or error.
        let db = Db::open(dir.path()).expect("second open");
        assert_eq!(user_version(&db), SCHEMA_VERSION);
    }

    #[test]
    fn open_locks_down_the_data_dir() {
        let dir = temp_dir();
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o755)).expect("widen dir");
        let _db = Db::open(dir.path()).expect("open");
        let mode = fs::metadata(dir.path())
            .expect("stat dir")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o700);
    }

    #[test]
    fn all_v1_tables_exist() {
        let dir = temp_dir();
        let db = Db::open(dir.path()).expect("open");
        let conn = db.conn.lock().expect("db lock poisoned");
        for table in ["history", "notes", "note_versions", "insights_daily"] {
            let count: i64 = conn
                .query_row(
                    "SELECT count(*) FROM sqlite_master WHERE type='table' AND name=?1",
                    [table],
                    |row| row.get(0),
                )
                .expect("query table");
            assert_eq!(count, 1, "table {table} missing");
        }
    }

    #[test]
    fn imports_legacy_history_on_first_open() {
        let dir = temp_dir();
        fs::write(
            dir.path().join("history.json"),
            r#"[
                {"id":"2-0","raw":"r2","text":"two words","modeId":"standard","polished":true,"at":2},
                {"id":"1-0","raw":"r1","text":"one","modeId":"standard","polished":false,"at":1}
            ]"#,
        )
        .expect("write json");
        let db = Db::open(dir.path()).expect("open");
        let rows = db.history_list().expect("list");
        assert_eq!(rows.len(), 2);
        // Newest first by `at`.
        assert_eq!(rows[0].id, "2-0");
        assert_eq!(rows[0].word_count, 2);
        assert!(rows[0].used_ai);
        assert_eq!(rows[0].raw_text, "r2");
        assert!(rows[0].app_name.is_none());
        assert!(rows[0].duration_ms.is_none());
        assert!(!rows[1].used_ai);
        // The file is preserved after import.
        assert!(dir.path().join("history.json").exists());
    }

    #[test]
    fn malformed_legacy_history_is_skipped() {
        let dir = temp_dir();
        fs::write(dir.path().join("history.json"), "{ not valid json").expect("write json");
        let db = Db::open(dir.path()).expect("open despite bad json");
        assert_eq!(db.history_list().expect("list").len(), 0);
        // Corrupt file left in place, never deleted.
        assert!(dir.path().join("history.json").exists());
    }

    #[test]
    fn absent_legacy_history_is_fine() {
        let dir = temp_dir();
        let db = Db::open(dir.path()).expect("open");
        assert_eq!(db.history_list().expect("list").len(), 0);
    }

    #[test]
    fn append_list_delete_clear_roundtrip() {
        let dir = temp_dir();
        let db = Db::open(dir.path()).expect("open");
        db.history_append(
            "a",
            10,
            "first",
            "raw1",
            "standard",
            Some("Mail"),
            Some(500),
            1,
            false,
        )
        .expect("append a");
        db.history_append("b", 20, "second", "raw2", "standard", None, None, 1, true)
            .expect("append b");
        let rows = db.history_list().expect("list");
        assert_eq!(rows.len(), 2);
        // Newest first.
        assert_eq!(rows[0].id, "b");
        assert_eq!(rows[1].id, "a");
        assert_eq!(rows[1].app_name.as_deref(), Some("Mail"));
        assert_eq!(rows[1].duration_ms, Some(500));

        db.history_delete("a").expect("delete a");
        let rows = db.history_list().expect("list after delete");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, "b");
        // Deleting a missing id is a no-op, not an error.
        db.history_delete("missing").expect("delete missing");

        db.history_clear().expect("clear");
        assert_eq!(db.history_list().expect("list after clear").len(), 0);
    }

    #[test]
    fn append_enforces_cap() {
        let dir = temp_dir();
        let db = Db::open(dir.path()).expect("open");
        let total = HISTORY_CAP + 5;
        for i in 0..total {
            db.history_append(
                &format!("id-{i}"),
                i,
                "text",
                "raw",
                "standard",
                None,
                None,
                1,
                false,
            )
            .expect("append");
        }
        let rows = db.history_list().expect("list");
        assert_eq!(rows.len() as i64, HISTORY_CAP);
        // The newest row survived; the oldest were dropped.
        assert_eq!(rows[0].id, format!("id-{}", total - 1));
        assert_eq!(
            rows.last().expect("last").id,
            format!("id-{}", total - HISTORY_CAP)
        );
    }

    #[test]
    fn corrupt_db_is_renamed_aside_and_fresh_db_works() {
        let dir = temp_dir();
        let db_path = dir.path().join("velata.db");
        // A non-SQLite file: opening then querying it fails the migration probe.
        fs::write(&db_path, b"this is not a sqlite database at all").expect("write garbage");
        let db = Db::open(dir.path()).expect("open recovers from corrupt file");
        // A `.corrupt-` sibling now holds the bad bytes.
        let corrupt = fs::read_dir(dir.path())
            .expect("read dir")
            .filter_map(Result::ok)
            .any(|e| {
                e.file_name()
                    .to_string_lossy()
                    .contains("velata.db.corrupt-")
            });
        assert!(corrupt, "corrupt file was not preserved aside");
        // The fresh DB is usable.
        db.history_append("x", 1, "t", "r", "standard", None, None, 1, false)
            .expect("append to fresh db");
        assert_eq!(db.history_list().expect("list").len(), 1);
    }
}
