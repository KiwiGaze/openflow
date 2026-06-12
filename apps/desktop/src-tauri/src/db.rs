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
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::Connection;
use serde::Deserialize;

use crate::error::{AppError, AppResult};
use crate::notes::{Note, NoteSummary, NoteVersion, MAX_NOTE_CONTENT_BYTES, NOTE_VERSION_CAP};

/// Highest schema version this build knows how to run. Each step is applied in
/// order; the runner stops here.
const SCHEMA_VERSION: i64 = 1;

/// Owns the single SQLite connection behind a `Mutex` (rusqlite's `Connection`
/// is not `Sync`). Cheap to clone-share via `Arc<Db>`.
pub struct Db {
    conn: Mutex<Connection>,
    /// Disambiguates note and version ids minted within the same millisecond
    /// (the `${ms}-${seq}` scheme history uses).
    note_seq: AtomicU64,
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
            note_seq: AtomicU64::new(0),
        })
    }

    /// Mints a `${ms}-${seq}` id (the scheme history uses). The sequence only
    /// disambiguates two ids minted in the same millisecond.
    fn next_id(&self, at: i64) -> String {
        let seq = self.note_seq.fetch_add(1, Ordering::SeqCst);
        format!("{at}-{seq}")
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

    /// All-time words per app from the history table, highest first. Rows with
    /// no recorded app (legacy imports) are excluded so the breakdown only ever
    /// shows real app names. Caller passes the row cap.
    pub fn history_per_app(&self, limit: i64) -> AppResult<Vec<(String, u64)>> {
        let conn = self.conn.lock().expect("db lock poisoned");
        let mut stmt = conn.prepare(
            "SELECT app_name, SUM(word_count) FROM history \
             WHERE app_name IS NOT NULL \
             GROUP BY app_name ORDER BY 2 DESC LIMIT ?1",
        )?;
        let rows = stmt
            .query_map([limit], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?.max(0) as u64,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Accumulates one dictation into the local-day `insights_daily` row. Only
    /// ever called when `app_stats_enabled` (the opt-in persistence gate lives
    /// in the pipeline); a single UPSERT so two dictations on the same day sum.
    pub fn insights_upsert_daily(
        &self,
        day: &str,
        words: i64,
        used_ai: bool,
        fixes: i64,
        duration_ms: i64,
    ) -> AppResult<()> {
        let conn = self.conn.lock().expect("db lock poisoned");
        conn.execute(
            "INSERT INTO insights_daily \
             (day, words, dictations, ai_dictations, fixes, duration_ms) \
             VALUES (?1, ?2, 1, ?3, ?4, ?5) \
             ON CONFLICT(day) DO UPDATE SET \
                words = words + excluded.words, \
                dictations = dictations + 1, \
                ai_dictations = ai_dictations + excluded.ai_dictations, \
                fixes = fixes + excluded.fixes, \
                duration_ms = duration_ms + excluded.duration_ms",
            rusqlite::params![day, words, used_ai as i64, fixes, duration_ms],
        )?;
        Ok(())
    }

    /// All-time totals summed over `insights_daily`. `None` when no day rows
    /// exist yet, so the caller renders "no all-time data" rather than zeros.
    pub fn insights_totals(&self) -> AppResult<Option<InsightsTotals>> {
        let conn = self.conn.lock().expect("db lock poisoned");
        // COUNT distinguishes "no rows" (None) from "rows that sum to zero"; the
        // SUMs are NULL with no rows, so coalesce them to 0 for the populated case.
        conn.query_row(
            "SELECT COUNT(*), \
                    COALESCE(SUM(words), 0), \
                    COALESCE(SUM(dictations), 0), \
                    COALESCE(SUM(ai_dictations), 0), \
                    COALESCE(SUM(fixes), 0), \
                    COALESCE(SUM(duration_ms), 0) \
             FROM insights_daily",
            [],
            |row| {
                let days: i64 = row.get(0)?;
                if days == 0 {
                    return Ok(None);
                }
                Ok(Some(InsightsTotals {
                    words: row.get::<_, i64>(1)?.max(0) as u64,
                    dictations: row.get::<_, i64>(2)?.max(0) as u64,
                    ai_dictations: row.get::<_, i64>(3)?.max(0) as u64,
                    fixes: row.get::<_, i64>(4)?.max(0) as u64,
                    duration_ms: row.get::<_, i64>(5)?.max(0) as u64,
                }))
            },
        )
        .map_err(Into::into)
    }

    /// Every day that has an `insights_daily` row, as sorted `YYYY-MM-DD`
    /// strings — the streak calculator's input.
    pub fn insights_days(&self) -> AppResult<Vec<String>> {
        let conn = self.conn.lock().expect("db lock poisoned");
        let mut stmt = conn.prepare("SELECT day FROM insights_daily ORDER BY day ASC")?;
        let days = stmt
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(days)
    }

    /// Removes every `insights_daily` row (the "reset all-time stats" action).
    pub fn insights_clear(&self) -> AppResult<()> {
        let conn = self.conn.lock().expect("db lock poisoned");
        conn.execute("DELETE FROM insights_daily", [])?;
        Ok(())
    }

    // ---- Scratchpad notes (text only — never audio) ------------------------

    /// Non-deleted notes for the list, pinned first then most-recently updated.
    /// `search` filters case-insensitively over title and body (LIKE), so the
    /// caller can wire a single search box to both fields. Previews are derived
    /// here with the shared tag-strip so the list never ships raw HTML.
    pub fn notes_list(&self, search: Option<&str>) -> AppResult<Vec<NoteSummary>> {
        let conn = self.conn.lock().expect("db lock poisoned");
        // A NULL search means "no filter"; an explicit `%term%` is built once so
        // the bound parameter stays parameterized (no string interpolation).
        let pattern = search
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| format!("%{}%", escape_like(s)));
        let mut stmt = conn.prepare(
            "SELECT id, title, content, updated_at, pinned FROM notes \
             WHERE deleted_at IS NULL \
               AND (?1 IS NULL OR title LIKE ?1 ESCAPE '\\' OR content LIKE ?1 ESCAPE '\\') \
             ORDER BY pinned DESC, updated_at DESC",
        )?;
        let rows = stmt
            .query_map([pattern], |row| {
                let content: String = row.get(2)?;
                Ok(NoteSummary {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    preview: crate::notes::preview(&content),
                    updated_at: row.get(3)?,
                    pinned: row.get::<_, i64>(4)? != 0,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// A single non-deleted note by id, or `None` if missing or deleted.
    pub fn note_get(&self, id: &str) -> AppResult<Option<Note>> {
        let conn = self.conn.lock().expect("db lock poisoned");
        let note = conn
            .query_row(
                "SELECT id, title, content, created_at, updated_at, pinned FROM notes \
                 WHERE id = ?1 AND deleted_at IS NULL",
                [id],
                row_to_note,
            )
            .map(Some)
            .or_else(|err| match err {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })?;
        Ok(note)
    }

    /// Creates an empty note (no title, no body) with timestamps set to now.
    pub fn note_create(&self) -> AppResult<Note> {
        let now = now_ms();
        let id = self.next_id(now);
        let conn = self.conn.lock().expect("db lock poisoned");
        conn.execute(
            "INSERT INTO notes (id, title, content, created_at, updated_at, pinned, deleted_at) \
             VALUES (?1, '', '', ?2, ?2, 0, NULL)",
            rusqlite::params![id, now],
        )?;
        Ok(Note {
            id,
            title: String::new(),
            content: String::new(),
            created_at: now,
            updated_at: now,
            pinned: false,
        })
    }

    /// Updates a note's title and body and bumps `updated_at`. A body over the
    /// 1 MB guard is rejected — never truncated, so the user's words are safe.
    pub fn note_update(&self, id: &str, title: &str, content: &str) -> AppResult<()> {
        if content.len() > MAX_NOTE_CONTENT_BYTES {
            return Err(AppError::Settings(
                "This note is too large to save. Shorten it and try again.".into(),
            ));
        }
        let conn = self.conn.lock().expect("db lock poisoned");
        conn.execute(
            "UPDATE notes SET title = ?2, content = ?3, updated_at = ?4 \
             WHERE id = ?1 AND deleted_at IS NULL",
            rusqlite::params![id, title, content, now_ms()],
        )?;
        Ok(())
    }

    /// Pins or unpins a note (pinned notes sort to the top of the list).
    pub fn note_set_pinned(&self, id: &str, pinned: bool) -> AppResult<()> {
        let conn = self.conn.lock().expect("db lock poisoned");
        conn.execute(
            "UPDATE notes SET pinned = ?2, updated_at = ?3 \
             WHERE id = ?1 AND deleted_at IS NULL",
            rusqlite::params![id, pinned as i64, now_ms()],
        )?;
        Ok(())
    }

    /// Soft-deletes a note: sets `deleted_at` so it leaves the list but its
    /// bytes (and versions) stay on disk. User data is never destroyed.
    pub fn note_soft_delete(&self, id: &str) -> AppResult<()> {
        let conn = self.conn.lock().expect("db lock poisoned");
        conn.execute(
            "UPDATE notes SET deleted_at = ?2 WHERE id = ?1 AND deleted_at IS NULL",
            rusqlite::params![id, now_ms()],
        )?;
        Ok(())
    }

    /// Every version of a note, newest first.
    pub fn note_versions(&self, note_id: &str) -> AppResult<Vec<NoteVersion>> {
        let conn = self.conn.lock().expect("db lock poisoned");
        let mut stmt = conn.prepare(
            "SELECT id, note_id, content, source, transform_id, created_at \
             FROM note_versions WHERE note_id = ?1 ORDER BY created_at DESC, id DESC",
        )?;
        let rows = stmt
            .query_map([note_id], row_to_version)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Records a version snapshot, then prunes the note's versions back to the
    /// newest [`NOTE_VERSION_CAP`] (a safety net, not an archive).
    pub fn note_version_add(
        &self,
        note_id: &str,
        content: &str,
        source: &str,
        transform_id: Option<&str>,
    ) -> AppResult<()> {
        let now = now_ms();
        let id = self.next_id(now);
        let conn = self.conn.lock().expect("db lock poisoned");
        conn.execute(
            "INSERT INTO note_versions (id, note_id, content, source, transform_id, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![id, note_id, content, source, transform_id, now],
        )?;
        conn.execute(
            "DELETE FROM note_versions WHERE note_id = ?1 AND id NOT IN \
             (SELECT id FROM note_versions WHERE note_id = ?1 \
              ORDER BY created_at DESC, id DESC LIMIT ?2)",
            rusqlite::params![note_id, NOTE_VERSION_CAP],
        )?;
        Ok(())
    }

    /// A single version by id (the body the caller restores), or `None`.
    pub fn note_version_get(&self, version_id: &str) -> AppResult<Option<NoteVersion>> {
        let conn = self.conn.lock().expect("db lock poisoned");
        let version = conn
            .query_row(
                "SELECT id, note_id, content, source, transform_id, created_at \
                 FROM note_versions WHERE id = ?1",
                [version_id],
                row_to_version,
            )
            .map(Some)
            .or_else(|err| match err {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })?;
        Ok(version)
    }
}

fn row_to_note(row: &rusqlite::Row) -> rusqlite::Result<Note> {
    Ok(Note {
        id: row.get(0)?,
        title: row.get(1)?,
        content: row.get(2)?,
        created_at: row.get(3)?,
        updated_at: row.get(4)?,
        pinned: row.get::<_, i64>(5)? != 0,
    })
}

fn row_to_version(row: &rusqlite::Row) -> rusqlite::Result<NoteVersion> {
    Ok(NoteVersion {
        id: row.get(0)?,
        note_id: row.get(1)?,
        content: row.get(2)?,
        source: row.get(3)?,
        transform_id: row.get(4)?,
        created_at: row.get(5)?,
    })
}

/// Escapes the LIKE wildcards in a user search term so a literal `%` or `_`
/// matches itself (paired with `ESCAPE '\'` in the query).
fn escape_like(term: &str) -> String {
    term.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

/// Unix epoch milliseconds — the timestamp every note row and version stores.
fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// All-time usage totals summed over `insights_daily`. Counts and durations
/// only — never words or audio. The IPC-facing shape is assembled in
/// `commands.rs`; this stays storage-shaped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InsightsTotals {
    pub words: u64,
    pub dictations: u64,
    pub ai_dictations: u64,
    pub fixes: u64,
    pub duration_ms: u64,
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
    fn insights_upsert_accumulates_per_day() {
        let dir = temp_dir();
        let db = Db::open(dir.path()).expect("open");
        // Two dictations on the same day sum into one row.
        db.insights_upsert_daily("2026-06-13", 10, true, 2, 60_000)
            .expect("upsert 1");
        db.insights_upsert_daily("2026-06-13", 5, false, 1, 30_000)
            .expect("upsert 2");
        // A different day is a separate row.
        db.insights_upsert_daily("2026-06-12", 7, true, 0, 20_000)
            .expect("upsert other day");

        let days = db.insights_days().expect("days");
        assert_eq!(days, vec!["2026-06-12", "2026-06-13"]);

        let totals = db.insights_totals().expect("totals").expect("some totals");
        assert_eq!(totals.words, 22);
        assert_eq!(totals.dictations, 3);
        assert_eq!(totals.ai_dictations, 2);
        assert_eq!(totals.fixes, 3);
        assert_eq!(totals.duration_ms, 110_000);
    }

    #[test]
    fn insights_totals_empty_is_none_then_cleared() {
        let dir = temp_dir();
        let db = Db::open(dir.path()).expect("open");
        // No rows yet → None, so the UI shows "no all-time data" not zeros.
        assert!(db.insights_totals().expect("totals").is_none());
        assert!(db.insights_days().expect("days").is_empty());

        db.insights_upsert_daily("2026-06-13", 3, false, 0, 0)
            .expect("upsert");
        assert!(db.insights_totals().expect("totals").is_some());

        db.insights_clear().expect("clear");
        assert!(db.insights_totals().expect("totals").is_none());
        assert!(db.insights_days().expect("days").is_empty());
    }

    #[test]
    fn history_per_app_aggregates_and_excludes_null_app() {
        let dir = temp_dir();
        let db = Db::open(dir.path()).expect("open");
        db.history_append("1", 1, "t", "r", "standard", Some("Mail"), None, 3, false)
            .expect("append 1");
        db.history_append("2", 2, "t", "r", "standard", Some("Mail"), None, 2, false)
            .expect("append 2");
        db.history_append("3", 3, "t", "r", "standard", Some("Notes"), None, 4, false)
            .expect("append 3");
        // No app recorded → must be excluded from the breakdown.
        db.history_append("4", 4, "t", "r", "standard", None, None, 99, false)
            .expect("append 4");

        let per_app = db.history_per_app(10).expect("per app");
        // Highest words first: Mail (3+2=5) then Notes (4); the null-app row is gone.
        assert_eq!(
            per_app,
            vec![("Mail".to_string(), 5), ("Notes".to_string(), 4)]
        );
        // The limit bounds the result.
        assert_eq!(
            db.history_per_app(1).expect("per app limit"),
            vec![("Mail".to_string(), 5)]
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

    #[test]
    fn notes_crud_roundtrip() {
        let dir = temp_dir();
        let db = Db::open(dir.path()).expect("open");
        let note = db.note_create().expect("create");
        assert!(note.title.is_empty());
        assert!(note.content.is_empty());
        assert!(!note.pinned);

        db.note_update(&note.id, "Title", "<p>Body</p>")
            .expect("update");
        let got = db.note_get(&note.id).expect("get").expect("present");
        assert_eq!(got.title, "Title");
        assert_eq!(got.content, "<p>Body</p>");
        // The update bumped updated_at past created_at-or-equal.
        assert!(got.updated_at >= got.created_at);

        let list = db.notes_list(None).expect("list");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, note.id);
        // The preview is the stripped body, not raw HTML.
        assert_eq!(list[0].preview, "Body");

        db.note_set_pinned(&note.id, true).expect("pin");
        assert!(db.note_get(&note.id).expect("get").expect("present").pinned);
    }

    #[test]
    fn soft_delete_hides_from_list_and_get() {
        let dir = temp_dir();
        let db = Db::open(dir.path()).expect("open");
        let note = db.note_create().expect("create");
        db.note_soft_delete(&note.id).expect("soft delete");
        assert!(db.note_get(&note.id).expect("get").is_none());
        assert!(db.notes_list(None).expect("list").is_empty());
        // The row still exists (soft delete only) — a second delete is a no-op.
        db.note_soft_delete(&note.id).expect("second delete no-op");
    }

    #[test]
    fn notes_search_matches_title_and_content_case_insensitively() {
        let dir = temp_dir();
        let db = Db::open(dir.path()).expect("open");
        let a = db.note_create().expect("create a");
        db.note_update(&a.id, "Groceries", "<p>milk and eggs</p>")
            .expect("update a");
        let b = db.note_create().expect("create b");
        db.note_update(&b.id, "Meeting", "<p>discuss the budget</p>")
            .expect("update b");

        // Title match, case-insensitive.
        let by_title = db.notes_list(Some("grocer")).expect("search title");
        assert_eq!(by_title.len(), 1);
        assert_eq!(by_title[0].id, a.id);

        // Content match.
        let by_content = db.notes_list(Some("BUDGET")).expect("search content");
        assert_eq!(by_content.len(), 1);
        assert_eq!(by_content[0].id, b.id);

        // Blank search returns everything.
        assert_eq!(db.notes_list(Some("   ")).expect("blank").len(), 2);
        // No match returns nothing.
        assert!(db.notes_list(Some("zzz")).expect("none").is_empty());
    }

    #[test]
    fn notes_list_orders_pinned_first_then_recent() {
        let dir = temp_dir();
        let db = Db::open(dir.path()).expect("open");
        let first = db.note_create().expect("first");
        let second = db.note_create().expect("second");
        let third = db.note_create().expect("third");
        // Touch `second` last so it is the most-recently updated.
        db.note_update(&first.id, "first", "").expect("touch first");
        db.note_update(&third.id, "third", "").expect("touch third");
        db.note_update(&second.id, "second", "")
            .expect("touch second");
        // Pin the oldest-updated note; it must still lead the list.
        db.note_set_pinned(&first.id, true).expect("pin first");

        let list = db.notes_list(None).expect("list");
        assert_eq!(list[0].id, first.id, "pinned leads regardless of recency");
        assert!(list[0].pinned);
        // The rest follow most-recent-updated first.
        assert_eq!(list[1].id, second.id);
        assert_eq!(list[2].id, third.id);
    }

    #[test]
    fn note_versions_add_list_get_newest_first_and_capped() {
        let dir = temp_dir();
        let db = Db::open(dir.path()).expect("open");
        let note = db.note_create().expect("create");
        // Insert more than the cap; only the newest NOTE_VERSION_CAP survive.
        let total = NOTE_VERSION_CAP + 5;
        for i in 0..total {
            db.note_version_add(&note.id, &format!("v{i}"), "transform", Some("t1"))
                .expect("add version");
        }
        let versions = db.note_versions(&note.id).expect("list versions");
        assert_eq!(versions.len() as i64, NOTE_VERSION_CAP);
        // Newest first: the last inserted leads.
        assert_eq!(versions[0].content, format!("v{}", total - 1));
        assert_eq!(versions[0].source, "transform");
        assert_eq!(versions[0].transform_id.as_deref(), Some("t1"));
        // The oldest survivor is `total - cap`, so earlier ones were pruned.
        assert_eq!(
            versions.last().expect("last").content,
            format!("v{}", total - NOTE_VERSION_CAP)
        );

        // Version-get returns the snapshot by id.
        let head = &versions[0];
        let fetched = db
            .note_version_get(&head.id)
            .expect("get version")
            .expect("present");
        assert_eq!(fetched.content, head.content);
        assert!(db.note_version_get("missing").expect("get").is_none());
    }

    #[test]
    fn restore_sequence_snapshots_current_then_swaps_content() {
        // The command-level restore is: snapshot the CURRENT body as a "restore"
        // version, then set the note's content to the chosen version's. This
        // exercises that db sequence directly (the command just orchestrates it).
        let dir = temp_dir();
        let db = Db::open(dir.path()).expect("open");
        let note = db.note_create().expect("create");
        db.note_update(&note.id, "t", "<p>original</p>")
            .expect("write original");
        db.note_version_add(&note.id, "<p>original</p>", "created", None)
            .expect("snapshot original");
        db.note_update(&note.id, "t", "<p>edited</p>")
            .expect("edit");

        // Restore the "original" snapshot: first snapshot current ("edited").
        let original = db
            .note_versions(&note.id)
            .expect("versions")
            .into_iter()
            .find(|v| v.content == "<p>original</p>")
            .expect("original snapshot");
        let current = db
            .note_get(&note.id)
            .expect("get")
            .expect("present")
            .content;
        db.note_version_add(&note.id, &current, "restore", None)
            .expect("snapshot before restore");
        db.note_update(&note.id, "t", &original.content)
            .expect("apply restore");

        assert_eq!(
            db.note_get(&note.id)
                .expect("get")
                .expect("present")
                .content,
            "<p>original</p>"
        );
        // The pre-restore body is recoverable as a "restore" version.
        assert!(db
            .note_versions(&note.id)
            .expect("versions")
            .iter()
            .any(|v| v.source == "restore" && v.content == "<p>edited</p>"));
    }

    #[test]
    fn note_update_rejects_oversized_content() {
        let dir = temp_dir();
        let db = Db::open(dir.path()).expect("open");
        let note = db.note_create().expect("create");
        let too_big = "a".repeat(MAX_NOTE_CONTENT_BYTES + 1);
        assert!(db.note_update(&note.id, "t", &too_big).is_err());
        // The note was not partially written — content stays empty.
        assert_eq!(
            db.note_get(&note.id)
                .expect("get")
                .expect("present")
                .content,
            ""
        );
        // Exactly at the limit is allowed.
        let at_limit = "a".repeat(MAX_NOTE_CONTENT_BYTES);
        assert!(db.note_update(&note.id, "t", &at_limit).is_ok());
    }
}
