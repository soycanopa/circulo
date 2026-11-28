//! Local storage, owned exclusively by the core.
//!
//! SQLite holds projects, sessions, turns and parts. Nothing leaves the
//! machine, and nothing here knows about GPUI, the daemon or a provider: the
//! core decides what to write, this crate knows how it is stored.
//!
//! One row per turn (`prompt` plus its position in the session) and one row per
//! part (`Part` as JSON, in the order the driver emitted it). That is exactly
//! what the transcript needs to draw a session again after a restart.
//!
//! `docs/implement.md` Phase 1, item 8. Settings (JSON) and the attachment blob
//! store join this crate when they land; they are not part of this schema.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

use circulo_protocol::{AttachmentSummary, Part};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::Value;
use uuid::Uuid;

/// Where the database lives when `CIRCULO_HOME` is not set.
const HOME_DIRECTORY: &str = ".circulo";
const DATABASE_FILE: &str = "circulo.db";

/// Schema version, tracked in SQLite's own `user_version`.
///
/// 1 created the tables. 2 added `sessions.provider_session`, the provider's
/// own key for a session, so a restart can hand it back and the agent resumes
/// where it left off. 3 added `settings`, the client's own key/value store —
/// the model favourites, and whatever the interface remembers next. 4 added
/// `projects`, the folders a session can belong to.
const SCHEMA_VERSION: i64 = 5;

/// The database path: `CIRCULO_HOME/circulo.db`, or `~/.circulo/circulo.db`.
///
/// The override exists so tests and a second profile never touch the real one.
pub fn default_path() -> Result<PathBuf, StoreError> {
    home_path(std::env::var_os("CIRCULO_HOME"), std::env::var_os("HOME"))
}

/// The decision itself, separated from the environment so it can be tested
/// without mutating process-wide state (which is unsafe in this edition).
fn home_path(
    circulo_home: Option<std::ffi::OsString>,
    home: Option<std::ffi::OsString>,
) -> Result<PathBuf, StoreError> {
    let directory =
        match circulo_home {
            Some(directory) => PathBuf::from(directory),
            None => PathBuf::from(home.ok_or_else(|| {
                StoreError::Invalid("neither CIRCULO_HOME nor HOME is set".into())
            })?)
            .join(HOME_DIRECTORY),
        };

    Ok(directory.join(DATABASE_FILE))
}

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("storage: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("storage: {0}")]
    Json(#[from] serde_json::Error),
    #[error("storage: {0}")]
    Io(#[from] std::io::Error),
    #[error("storage: {0}")]
    Invalid(String),
}

/// A session as a list needs it: enough to show a row and open it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSummary {
    pub id: String,
    pub project: Option<String>,
    pub title: String,
    /// Seconds since the Unix epoch, bumped by every turn.
    pub updated_at: u64,
}

/// A folder the user added, shown in the sidebar and selectable for a new
/// session. The canonical path is the key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectRecord {
    pub path: String,
    pub name: String,
}

/// One turn of a session, ready to be drawn again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnRecord {
    pub turn: u64,
    pub prompt: String,
    pub parts: Vec<Part>,
    pub attachments: Vec<AttachmentSummary>,
}

/// The handle the core holds. One connection behind a mutex: writes come from
/// turn threads, reads from the connection that answers a client.
pub struct Store {
    connection: Mutex<Connection>,
    /// Where attachment bytes live: `<database dir>/blobs`. `None` for an
    /// in-memory store, which has no directory to own.
    blobs: Option<PathBuf>,
}

/// One attachment the store accepted: metadata plus the blob path the provider
/// can read. The id is the row the wire and `turn_attachments` refer to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredAttachment {
    pub id: String,
    pub name: String,
    pub mime: String,
    pub size: u64,
    pub path: PathBuf,
}

/// One file may not exceed this; the core applies a smaller total budget.
pub const MAX_ATTACHMENT_BYTES: u64 = 10 * 1024 * 1024;

impl Store {
    /// Opens the database, creating its directory and schema when missing.
    pub fn open(path: &Path) -> Result<Self, StoreError> {
        if let Some(directory) = path.parent() {
            std::fs::create_dir_all(directory)?;
        }

        let store = Self {
            connection: Mutex::new(Connection::open(path)?),
            blobs: path.parent().map(|directory| directory.join("blobs")),
        };
        store.migrate()?;
        Ok(store)
    }

    /// A throwaway database, for tests.
    pub fn in_memory() -> Result<Self, StoreError> {
        let store = Self {
            connection: Mutex::new(Connection::open_in_memory()?),
            blobs: None,
        };
        store.migrate()?;
        Ok(store)
    }

    fn migrate(&self) -> Result<(), StoreError> {
        let connection = self.connection();
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "foreign_keys", "ON")?;

        let version: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        if version >= SCHEMA_VERSION {
            return Ok(());
        }

        // Each step moves one version forward, so a database from any past
        // release reaches the current shape in order.
        if version < 1 {
            connection.execute_batch(
                r#"
                BEGIN;
                CREATE TABLE IF NOT EXISTS sessions (
                    id          TEXT PRIMARY KEY,
                    project     TEXT,
                    provider    TEXT NOT NULL,
                    title       TEXT NOT NULL DEFAULT '',
                    created_at  INTEGER NOT NULL,
                    updated_at  INTEGER NOT NULL
                );

                CREATE TABLE IF NOT EXISTS turns (
                    session_id  TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
                    turn        INTEGER NOT NULL,
                    prompt      TEXT NOT NULL,
                    created_at  INTEGER NOT NULL,
                    PRIMARY KEY (session_id, turn)
                );

                CREATE TABLE IF NOT EXISTS parts (
                    session_id  TEXT NOT NULL,
                    turn        INTEGER NOT NULL,
                    position    INTEGER NOT NULL,
                    payload     TEXT NOT NULL,
                    PRIMARY KEY (session_id, turn, position)
                );

                PRAGMA user_version = 1;
                COMMIT;
                "#,
            )?;
        }

        if version < 2 {
            connection.execute_batch(
                r#"
                ALTER TABLE sessions ADD COLUMN provider_session TEXT;
                PRAGMA user_version = 2;
                "#,
            )?;
        }

        if version < 3 {
            connection.execute_batch(
                r#"
                CREATE TABLE IF NOT EXISTS settings (
                    key   TEXT PRIMARY KEY,
                    value TEXT NOT NULL
                );
                PRAGMA user_version = 3;
                "#,
            )?;
        }

        if version < 4 {
            // A project is a folder; the path is its key. Existing sessions
            // already carry one, so the table starts with what they know
            // instead of an empty sidebar.
            let transaction = connection.unchecked_transaction()?;
            transaction.execute_batch(
                r#"
                CREATE TABLE IF NOT EXISTS projects (
                    path        TEXT PRIMARY KEY,
                    name        TEXT NOT NULL,
                    created_at  INTEGER NOT NULL
                );
                "#,
            )?;

            let paths = {
                let mut statement = transaction.prepare(
                    "SELECT DISTINCT project FROM sessions
                      WHERE project IS NOT NULL AND project <> ''",
                )?;
                statement
                    .query_map([], |row| row.get::<_, String>(0))?
                    .collect::<Result<Vec<_>, _>>()?
            };

            let created = now() as i64;
            for path in paths {
                transaction.execute(
                    "INSERT OR IGNORE INTO projects (path, name, created_at)
                     VALUES (?1, ?2, ?3)",
                    params![path, project_name(&path), created],
                )?;
            }

            transaction.pragma_update(None, "user_version", 4)?;
            transaction.commit()?;
        }

        if version < 5 {
            connection.execute_batch(
                r#"
                CREATE TABLE IF NOT EXISTS attachments (
                    id          TEXT PRIMARY KEY,
                    hash        TEXT NOT NULL,
                    name        TEXT NOT NULL,
                    mime        TEXT NOT NULL,
                    size        INTEGER NOT NULL,
                    created_at  INTEGER NOT NULL
                );
                CREATE INDEX IF NOT EXISTS attachments_hash ON attachments (hash);

                CREATE TABLE IF NOT EXISTS turn_attachments (
                    session_id     TEXT NOT NULL,
                    turn           INTEGER NOT NULL,
                    position       INTEGER NOT NULL,
                    attachment_id  TEXT NOT NULL REFERENCES attachments(id),
                    PRIMARY KEY (session_id, turn, position)
                );
                PRAGMA user_version = 5;
                "#,
            )?;
        }

        Ok(())
    }

    /// Records a new session. The title stays empty until the first prompt.
    pub fn start_session(
        &self,
        id: &str,
        project: Option<&str>,
        provider: &str,
    ) -> Result<(), StoreError> {
        let now = now();
        self.connection().execute(
            "INSERT INTO sessions (id, project, provider, title, created_at, updated_at)
             VALUES (?1, ?2, ?3, '', ?4, ?4)",
            params![id, project, provider, now as i64],
        )?;
        Ok(())
    }

    /// Records the prompt of a turn and moves the session to the top of the
    /// list. The first prompt also titles the session.
    pub fn start_turn(&self, session: &str, turn: u64, prompt: &str) -> Result<(), StoreError> {
        let now = now();
        let connection = self.connection();

        connection.execute(
            "INSERT INTO turns (session_id, turn, prompt, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![session, turn as i64, prompt, now as i64],
        )?;

        connection.execute(
            "UPDATE sessions
                SET updated_at = ?2,
                    title = CASE WHEN title = '' THEN ?3 ELSE title END
              WHERE id = ?1",
            params![session, now as i64, title_from(prompt)],
        )?;

        Ok(())
    }

    /// Appends one part to a turn, in the order the driver emitted it.
    pub fn append_part(
        &self,
        session: &str,
        turn: u64,
        position: u64,
        part: &Part,
    ) -> Result<(), StoreError> {
        let payload = serde_json::to_value(part)?;
        let payload = serde_json::to_string(&payload)?;

        self.connection().execute(
            "INSERT OR REPLACE INTO parts (session_id, turn, position, payload)
             VALUES (?1, ?2, ?3, ?4)",
            params![session, turn as i64, position as i64, payload],
        )?;
        Ok(())
    }

    /// Sessions, newest first. The sidebar will draw this list.
    pub fn sessions(&self) -> Result<Vec<SessionSummary>, StoreError> {
        let connection = self.connection();
        let mut statement = connection.prepare(
            "SELECT id, project, title, updated_at FROM sessions ORDER BY updated_at DESC",
        )?;

        let rows = statement.query_map([], |row| {
            Ok(SessionSummary {
                id: row.get(0)?,
                project: row.get(1)?,
                title: row.get(2)?,
                updated_at: row.get::<_, i64>(3)? as u64,
            })
        })?;

        let mut sessions = Vec::new();
        for row in rows {
            sessions.push(row?);
        }
        Ok(sessions)
    }

    pub fn has_session(&self, id: &str) -> Result<bool, StoreError> {
        let found: Option<i64> = self
            .connection()
            .query_row("SELECT 1 FROM sessions WHERE id = ?1", params![id], |row| {
                row.get(0)
            })
            .optional()?;
        Ok(found.is_some())
    }

    /// The project folder this session belongs to, if any. A missing session
    /// reads as `None` too; callers that care check `has_session` first.
    pub fn session_project(&self, id: &str) -> Result<Option<String>, StoreError> {
        let project = self
            .connection()
            .query_row(
                "SELECT project FROM sessions WHERE id = ?1",
                params![id],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()?
            .flatten();
        Ok(project)
    }

    /// The folders the user added, sorted for the sidebar.
    pub fn projects(&self) -> Result<Vec<ProjectRecord>, StoreError> {
        let connection = self.connection();
        let mut statement = connection
            .prepare("SELECT path, name FROM projects ORDER BY name COLLATE NOCASE, path")?;
        let rows = statement.query_map([], |row| {
            Ok(ProjectRecord {
                path: row.get(0)?,
                name: row.get(1)?,
            })
        })?;

        let mut projects = Vec::new();
        for row in rows {
            projects.push(row?);
        }
        Ok(projects)
    }

    /// Adds a project, or refreshes its name when the path is already known.
    pub fn add_project(&self, path: &str, name: &str) -> Result<(), StoreError> {
        self.connection().execute(
            "INSERT INTO projects (path, name, created_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(path) DO UPDATE SET name = excluded.name",
            params![path, name, now() as i64],
        )?;
        Ok(())
    }

    /// Removes a project from the list. Its sessions stay, and the folder is
    /// not touched: this is a sidebar entry, not a delete.
    pub fn remove_project(&self, path: &str) -> Result<(), StoreError> {
        self.connection()
            .execute("DELETE FROM projects WHERE path = ?1", params![path])?;
        Ok(())
    }

    /// The provider's own key for this session, from its last turn. `None`
    /// while no turn has run, or when the provider keeps no session of its own.
    pub fn provider_session(&self, id: &str) -> Result<Option<String>, StoreError> {
        let key = self
            .connection()
            .query_row(
                "SELECT provider_session FROM sessions WHERE id = ?1",
                params![id],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()?
            .flatten();
        Ok(key)
    }

    /// Remembers the key the provider handed back this turn, or forgets it
    /// with `None`: a stale key must not survive a provider that lost it.
    pub fn set_provider_session(&self, id: &str, key: Option<&str>) -> Result<(), StoreError> {
        self.connection().execute(
            "UPDATE sessions SET provider_session = ?2 WHERE id = ?1",
            params![id, key],
        )?;
        Ok(())
    }

    /// Trims a session to `keep_through` (inclusive): later turns and their
    /// parts are deleted. `None` removes every turn and clears the title. The
    /// provider key is always dropped, because after a rewind the agent context
    /// no longer matches the files and transcript.
    pub fn trim_turns(&self, session: &str, keep_through: Option<u64>) -> Result<(), StoreError> {
        let mut connection = self.connection();
        let transaction = connection.transaction()?;

        match keep_through {
            Some(turn) => {
                transaction.execute(
                    "DELETE FROM turns WHERE session_id = ?1 AND turn > ?2",
                    params![session, turn as i64],
                )?;
                transaction.execute(
                    "DELETE FROM parts WHERE session_id = ?1 AND turn > ?2",
                    params![session, turn as i64],
                )?;
                transaction.execute(
                    "DELETE FROM turn_attachments WHERE session_id = ?1 AND turn > ?2",
                    params![session, turn as i64],
                )?;
            }
            None => {
                transaction.execute("DELETE FROM turns WHERE session_id = ?1", params![session])?;
                transaction.execute("DELETE FROM parts WHERE session_id = ?1", params![session])?;
                transaction.execute(
                    "DELETE FROM turn_attachments WHERE session_id = ?1",
                    params![session],
                )?;
            }
        }

        transaction.execute(
            "UPDATE sessions
                SET provider_session = NULL,
                    title = CASE
                        WHEN EXISTS (SELECT 1 FROM turns WHERE session_id = ?1)
                        THEN title
                        ELSE ''
                    END,
                    updated_at = ?2
              WHERE id = ?1",
            params![session, now() as i64],
        )?;
        transaction.commit()?;
        Ok(())
    }

    /// The client's own value for `key`, if it ever set one. Settings are
    /// interface state — favourite models — so the shape is opaque here:
    /// whatever JSON went in comes back out.
    pub fn setting(&self, key: &str) -> Result<Option<Value>, StoreError> {
        let payload: Option<String> = self
            .connection()
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                params![key],
                |row| row.get(0),
            )
            .optional()?;
        Ok(payload
            .map(|payload| serde_json::from_str(&payload))
            .transpose()?)
    }

    /// Records the client's value for `key`, replacing whatever was there.
    pub fn set_setting(&self, key: &str, value: &Value) -> Result<(), StoreError> {
        let payload = serde_json::to_string(value)?;
        self.connection().execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, payload],
        )?;
        Ok(())
    }

    /// Everything the client ever stored, keyed. The map is small — favourite
    /// models today — so the wire carries it whole.
    pub fn settings(&self) -> Result<BTreeMap<String, Value>, StoreError> {
        let connection = self.connection();
        let mut statement = connection.prepare("SELECT key, value FROM settings")?;
        let rows = statement.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        let mut settings = BTreeMap::new();
        for row in rows {
            let (key, payload) = row?;
            settings.insert(key, serde_json::from_str(&payload)?);
        }
        Ok(settings)
    }

    /// Every turn of a session, in order, with its parts in order.
    /// Copies a file into the blob store and records it. The same bytes under
    /// the same name are written once; each call gets its own attachment row.
    pub fn put_attachment(&self, source: &Path) -> Result<StoredAttachment, StoreError> {
        let blobs = self.blobs.as_ref().ok_or_else(|| {
            StoreError::Invalid("this store has no attachment directory".to_owned())
        })?;

        let metadata = std::fs::metadata(source)?;
        if !metadata.is_file() {
            return Err(StoreError::Invalid(format!(
                "{} is not a file",
                source.display()
            )));
        }
        if metadata.len() > MAX_ATTACHMENT_BYTES {
            return Err(StoreError::Invalid(format!(
                "{} is larger than {} MiB",
                source.display(),
                MAX_ATTACHMENT_BYTES / (1024 * 1024)
            )));
        }

        let name = source
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.is_empty() && *name != "." && *name != "..")
            .ok_or_else(|| {
                StoreError::Invalid(format!("{} has no usable file name", source.display()))
            })?
            .to_owned();

        let bytes = std::fs::read(source)?;
        if bytes.len() as u64 > MAX_ATTACHMENT_BYTES {
            return Err(StoreError::Invalid(format!(
                "{} grew past {} MiB while being read",
                source.display(),
                MAX_ATTACHMENT_BYTES / (1024 * 1024)
            )));
        }

        let hash = blake3::hash(&bytes).to_hex().to_string();
        let directory = blobs.join(&hash);
        std::fs::create_dir_all(&directory)?;
        let path = directory.join(&name);
        if !path.exists() {
            std::fs::write(&path, &bytes)?;
        }

        let id = Uuid::new_v4().to_string();
        let mime = mime_for(&name);
        let size = bytes.len() as u64;
        self.connection().execute(
            "INSERT INTO attachments (id, hash, name, mime, size, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, hash, name, mime, size as i64, now() as i64],
        )?;

        Ok(StoredAttachment {
            id,
            name,
            mime,
            size,
            path,
        })
    }

    /// The attachment row and the blob path the provider can read.
    pub fn attachment(&self, id: &str) -> Result<Option<StoredAttachment>, StoreError> {
        let Some(blobs) = &self.blobs else {
            return Ok(None);
        };
        let row = self
            .connection()
            .query_row(
                "SELECT hash, name, mime, size FROM attachments WHERE id = ?1",
                params![id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, i64>(3)?,
                    ))
                },
            )
            .optional()?;

        Ok(row.map(|(hash, name, mime, size)| StoredAttachment {
            id: id.to_owned(),
            path: blobs.join(hash).join(&name),
            name,
            mime,
            size: size as u64,
        }))
    }

    /// Binds the attachments to the turn that carried them, in order.
    pub fn link_turn_attachments(
        &self,
        session: &str,
        turn: u64,
        attachments: &[AttachmentSummary],
    ) -> Result<(), StoreError> {
        let mut connection = self.connection();
        let transaction = connection.transaction()?;
        for (position, attachment) in attachments.iter().enumerate() {
            transaction.execute(
                "INSERT OR REPLACE INTO turn_attachments
                    (session_id, turn, position, attachment_id)
                 VALUES (?1, ?2, ?3, ?4)",
                params![session, turn as i64, position as i64, attachment.id],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    /// The attachments of one turn, in the order the user chose them.
    pub fn turn_attachments(
        &self,
        session: &str,
        turn: u64,
    ) -> Result<Vec<AttachmentSummary>, StoreError> {
        let connection = self.connection();
        let mut statement = connection.prepare(
            "SELECT a.id, a.name, a.mime, a.size
               FROM turn_attachments ta
               JOIN attachments a ON a.id = ta.attachment_id
              WHERE ta.session_id = ?1 AND ta.turn = ?2
              ORDER BY ta.position",
        )?;
        let rows = statement.query_map(params![session, turn as i64], |row| {
            Ok(AttachmentSummary {
                id: row.get(0)?,
                name: row.get(1)?,
                mime: row.get(2)?,
                size: row.get::<_, i64>(3)? as u64,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn transcript(&self, session: &str) -> Result<Vec<TurnRecord>, StoreError> {
        let connection = self.connection();
        let mut turns = Vec::new();

        let mut statement = connection
            .prepare("SELECT turn, prompt FROM turns WHERE session_id = ?1 ORDER BY turn")?;
        let rows = statement.query_map(params![session], |row| {
            Ok((row.get::<_, i64>(0)? as u64, row.get::<_, String>(1)?))
        })?;
        for row in rows {
            let (turn, prompt) = row?;
            turns.push(TurnRecord {
                turn,
                prompt,
                parts: Vec::new(),
                attachments: Vec::new(),
            });
        }

        let mut statement = connection.prepare(
            "SELECT turn, payload FROM parts WHERE session_id = ?1 ORDER BY turn, position",
        )?;
        let rows = statement.query_map(params![session], |row| {
            Ok((row.get::<_, i64>(0)? as u64, row.get::<_, String>(1)?))
        })?;
        for row in rows {
            let (turn, payload) = row?;
            let value: Value = serde_json::from_str(&payload)?;
            let part: Part = serde_json::from_value(value)?;
            if let Some(record) = turns.iter_mut().find(|record| record.turn == turn) {
                record.parts.push(part);
            }
        }

        let mut statement = connection.prepare(
            "SELECT ta.turn, a.id, a.name, a.mime, a.size
               FROM turn_attachments ta
               JOIN attachments a ON a.id = ta.attachment_id
              WHERE ta.session_id = ?1
              ORDER BY ta.turn, ta.position",
        )?;
        let rows = statement.query_map(params![session], |row| {
            Ok((
                row.get::<_, i64>(0)? as u64,
                AttachmentSummary {
                    id: row.get(1)?,
                    name: row.get(2)?,
                    mime: row.get(3)?,
                    size: row.get::<_, i64>(4)? as u64,
                },
            ))
        })?;
        for row in rows {
            let (turn, attachment) = row?;
            if let Some(record) = turns.iter_mut().find(|record| record.turn == turn) {
                record.attachments.push(attachment);
            }
        }

        Ok(turns)
    }

    fn connection(&self) -> MutexGuard<'_, Connection> {
        // A panicking thread must not take the daemon down with it.
        self.connection
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

/// The session title is the first prompt, cut at a word boundary near 60
/// characters so the sidebar row stays one line.
/// The MIME type from the extension. Unknown files stay octet-stream so the
/// core can refuse them instead of guessing.
fn mime_for(name: &str) -> String {
    let extension = Path::new(name)
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    let mime = match extension.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "txt" | "log" => "text/plain",
        "md" | "markdown" => "text/markdown",
        "json" => "application/json",
        "toml" => "application/toml",
        "yaml" | "yml" => "application/yaml",
        "xml" => "application/xml",
        "csv" => "text/csv",
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "rs" => "text/x-rust",
        "py" => "text/x-python",
        "js" => "text/javascript",
        "ts" => "text/typescript",
        "tsx" => "text/tsx",
        "jsx" => "text/jsx",
        "sh" | "bash" | "zsh" => "text/x-shellscript",
        "sql" => "text/x-sql",
        _ => "application/octet-stream",
    };

    mime.to_owned()
}

fn title_from(prompt: &str) -> String {
    let line = prompt.lines().next().unwrap_or_default().trim();
    if line.chars().count() <= 60 {
        return line.to_owned();
    }

    let cut: String = line.chars().take(60).collect();
    match cut.rsplit_once(' ') {
        Some((head, _)) if head.chars().count() > 30 => format!("{head}…"),
        _ => format!("{cut}…"),
    }
}

/// The display name of a project folder: its last component, or the whole path
/// when there is none (`/`).
fn project_name(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_owned())
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> Store {
        Store::in_memory().expect("in-memory store")
    }

    fn part(delta: &str) -> Part {
        Part::Text {
            delta: delta.to_owned(),
        }
    }

    #[test]
    fn a_session_starts_empty_and_is_listed() {
        let store = store();
        store
            .start_session("s1", Some("/tmp/project"), "demo")
            .expect("session");

        let sessions = store.sessions().expect("list");
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id, "s1");
        assert_eq!(sessions[0].project.as_deref(), Some("/tmp/project"));
        assert_eq!(sessions[0].title, "", "the first prompt titles it");
        assert!(store.has_session("s1").expect("has"));
        assert!(!store.has_session("nope").expect("has"));
        assert_eq!(
            store.session_project("s1").expect("project").as_deref(),
            Some("/tmp/project")
        );
        assert_eq!(
            store.session_project("nope").expect("project"),
            None,
            "a missing session reads as no project, like a session without one"
        );
    }

    #[test]
    fn the_first_prompt_titles_the_session() {
        let store = store();
        store.start_session("s1", None, "demo").expect("session");
        store
            .start_turn("s1", 1, "Lay out the transcript renderer, please")
            .expect("turn");

        assert_eq!(
            store.sessions().expect("list")[0].title,
            "Lay out the transcript renderer, please"
        );
    }

    #[test]
    fn a_long_title_is_cut_at_a_word() {
        let long = "a ".repeat(60) + "and then some more words that go on";
        let title = title_from(&long);
        assert!(title.chars().count() <= 62, "{title}");
        assert!(title.ends_with('…'));
    }

    #[test]
    fn a_transcript_round_trips_in_order() {
        let store = store();
        store.start_session("s1", None, "demo").expect("session");
        store.start_turn("s1", 1, "first").expect("turn");
        store
            .append_part("s1", 1, 0, &part("hello "))
            .expect("part");
        store.append_part("s1", 1, 1, &part("world")).expect("part");
        store.start_turn("s1", 2, "second").expect("turn");
        store
            .append_part(
                "s1",
                2,
                0,
                &Part::Reasoning {
                    delta: "thinking".into(),
                },
            )
            .expect("part");

        let turns = store.transcript("s1").expect("transcript");
        assert_eq!(turns.len(), 2);
        assert_eq!(turns[0].turn, 1);
        assert_eq!(turns[0].prompt, "first");
        assert_eq!(turns[0].parts, vec![part("hello "), part("world")]);
        assert_eq!(turns[1].prompt, "second");
        assert_eq!(
            turns[1].parts,
            vec![Part::Reasoning {
                delta: "thinking".into()
            }]
        );
    }

    #[test]
    fn reopening_a_database_keeps_its_sessions() {
        let directory =
            std::env::temp_dir().join(format!("circulo-store-{}-{}", std::process::id(), now()));
        let path = directory.join("circulo.db");

        {
            let store = Store::open(&path).expect("open");
            store.start_session("s1", None, "demo").expect("session");
            store.start_turn("s1", 1, "hello").expect("turn");
            store.append_part("s1", 1, 0, &part("hi")).expect("part");
        }

        let store = Store::open(&path).expect("reopen");
        assert_eq!(store.sessions().expect("list").len(), 1);
        assert_eq!(store.transcript("s1").expect("transcript").len(), 1);

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn the_provider_key_survives_a_reopen() {
        let directory = std::env::temp_dir().join(format!(
            "circulo-store-{}-{}-provider-key",
            std::process::id(),
            now()
        ));
        let path = directory.join("circulo.db");

        {
            let store = Store::open(&path).expect("open");
            store
                .start_session("s1", None, "opencode")
                .expect("session");
            assert_eq!(
                store.provider_session("s1").expect("key"),
                None,
                "a fresh session has no provider key"
            );
            store
                .set_provider_session("s1", Some("ses_acp_1"))
                .expect("set");
        }

        let store = Store::open(&path).expect("reopen");
        assert_eq!(
            store.provider_session("s1").expect("key").as_deref(),
            Some("ses_acp_1"),
            "the key outlives the process that wrote it"
        );

        // A provider that lost its session clears it, and the store forgets.
        store.set_provider_session("s1", None).expect("clear");
        assert_eq!(store.provider_session("s1").expect("key"), None);

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn a_version_1_database_migrates_in_place() {
        let directory = std::env::temp_dir().join(format!(
            "circulo-store-{}-{}-migration",
            std::process::id(),
            now()
        ));
        let path = directory.join("circulo.db");

        // A database from release v0.4: schema 1, no provider key column.
        {
            std::fs::create_dir_all(&directory).expect("directory");
            let connection = Connection::open(&path).expect("raw connection");
            connection
                .execute_batch(
                    r#"
                    CREATE TABLE sessions (
                        id          TEXT PRIMARY KEY,
                        project     TEXT,
                        provider    TEXT NOT NULL,
                        title       TEXT NOT NULL DEFAULT '',
                        created_at  INTEGER NOT NULL,
                        updated_at  INTEGER NOT NULL
                    );
                    CREATE TABLE turns (
                        session_id  TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
                        turn        INTEGER NOT NULL,
                        prompt      TEXT NOT NULL,
                        created_at  INTEGER NOT NULL,
                        PRIMARY KEY (session_id, turn)
                    );
                    CREATE TABLE parts (
                        session_id  TEXT NOT NULL,
                        turn        INTEGER NOT NULL,
                        position    INTEGER NOT NULL,
                        payload     TEXT NOT NULL,
                        PRIMARY KEY (session_id, turn, position)
                    );
                    PRAGMA user_version = 1;
                    "#,
                )
                .expect("schema 1");
            connection
                .execute(
                    "INSERT INTO sessions (id, project, provider, created_at, updated_at)
                     VALUES ('s1', '/tmp/project', 'demo', 0, 0)",
                    [],
                )
                .expect("row");
        }

        let store = Store::open(&path).expect("open migrates");
        assert_eq!(store.sessions().expect("list")[0].id, "s1", "data survived");
        assert_eq!(store.provider_session("s1").expect("key"), None);
        assert_eq!(
            store.projects().expect("projects"),
            vec![ProjectRecord {
                path: "/tmp/project".into(),
                name: "project".into(),
            }],
            "the migration backfills projects from the sessions that know one"
        );
        store
            .set_provider_session("s1", Some("ses_acp_1"))
            .expect("set");

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn trimming_a_session_removes_later_turns_and_the_provider_key() {
        let store = store();
        store.start_session("s1", None, "demo").expect("session");
        for turn in 1..=3 {
            store
                .start_turn("s1", turn, &format!("prompt {turn}"))
                .expect("turn");
            store
                .append_part("s1", turn, 0, &part(&format!("part {turn}")))
                .expect("part");
        }
        store
            .set_provider_session("s1", Some("ses_acp_1"))
            .expect("key");

        store.trim_turns("s1", Some(1)).expect("trim");
        let transcript = store.transcript("s1").expect("transcript");
        assert_eq!(transcript.len(), 1);
        assert_eq!(transcript[0].turn, 1);
        assert_eq!(transcript[0].parts, vec![part("part 1")]);
        assert_eq!(store.sessions().expect("list")[0].title, "prompt 1");
        assert_eq!(
            store.provider_session("s1").expect("key"),
            None,
            "the agent context no longer matches the files"
        );

        store.trim_turns("s1", None).expect("trim all");
        assert!(store.transcript("s1").expect("transcript").is_empty());
        assert_eq!(
            store.sessions().expect("list")[0].title,
            "",
            "no prompt left to title the session"
        );
    }

    #[test]
    fn projects_are_added_listed_and_removed() {
        let store = store();

        store.add_project("/tmp/zeta", "zeta").expect("add zeta");
        store.add_project("/tmp/alpha", "alpha").expect("add alpha");
        assert_eq!(
            store
                .projects()
                .expect("list")
                .into_iter()
                .map(|project| project.name)
                .collect::<Vec<_>>(),
            vec!["alpha", "zeta"],
            "the sidebar gets a stable order"
        );

        // Adding the same path again refreshes the name instead of duplicating.
        store
            .add_project("/tmp/alpha", "Alpha Renamed")
            .expect("add again");
        let projects = store.projects().expect("list");
        assert_eq!(projects.len(), 2);
        assert_eq!(projects[0].name, "Alpha Renamed");

        store.remove_project("/tmp/alpha").expect("remove");
        assert_eq!(
            store.projects().expect("list"),
            vec![ProjectRecord {
                path: "/tmp/zeta".into(),
                name: "zeta".into(),
            }],
            "removing a project does not touch its sessions"
        );
    }

    #[test]
    fn the_home_override_decides_the_path() {
        let path = home_path(Some("/tmp/circulo-home".into()), None).expect("path");
        assert_eq!(path, PathBuf::from("/tmp/circulo-home/circulo.db"));

        let path = home_path(None, Some("/home/me".into())).expect("path");
        assert_eq!(path, PathBuf::from("/home/me/.circulo/circulo.db"));

        assert!(home_path(None, None).is_err());
    }

    #[test]
    fn a_setting_round_trips_and_survives_a_reopen() {
        let directory = std::env::temp_dir().join(format!(
            "circulo-store-{}-{}-settings",
            std::process::id(),
            now()
        ));
        let path = directory.join("circulo.db");

        {
            let store = Store::open(&path).expect("open");
            assert_eq!(store.setting("favorites.model").expect("read"), None);
            store
                .set_setting("favorites.model", &serde_json::json!(["a", "b"]))
                .expect("write");
        }

        let store = Store::open(&path).expect("reopen");
        assert_eq!(
            store.setting("favorites.model").expect("read"),
            Some(serde_json::json!(["a", "b"])),
            "the setting outlives the process"
        );
        store
            .set_setting("favorites.model", &serde_json::json!(["c"]))
            .expect("replace");
        assert_eq!(
            store.setting("favorites.model").expect("read"),
            Some(serde_json::json!(["c"]))
        );

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn a_version_2_database_gains_the_settings_table() {
        let directory = std::env::temp_dir().join(format!(
            "circulo-store-{}-{}-settings-migration",
            std::process::id(),
            now()
        ));
        let path = directory.join("circulo.db");

        // A database from the continuity release: schema 2, no settings.
        {
            std::fs::create_dir_all(&directory).expect("directory");
            let connection = Connection::open(&path).expect("raw connection");
            connection
                .execute_batch(
                    r#"
                    CREATE TABLE sessions (
                        id          TEXT PRIMARY KEY,
                        project     TEXT,
                        provider    TEXT NOT NULL,
                        title       TEXT NOT NULL DEFAULT '',
                        created_at  INTEGER NOT NULL,
                        updated_at  INTEGER NOT NULL,
                        provider_session TEXT
                    );
                    CREATE TABLE turns (
                        session_id  TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
                        turn        INTEGER NOT NULL,
                        prompt      TEXT NOT NULL,
                        created_at  INTEGER NOT NULL,
                        PRIMARY KEY (session_id, turn)
                    );
                    CREATE TABLE parts (
                        session_id  TEXT NOT NULL,
                        turn        INTEGER NOT NULL,
                        position    INTEGER NOT NULL,
                        payload     TEXT NOT NULL,
                        PRIMARY KEY (session_id, turn, position)
                    );
                    PRAGMA user_version = 2;
                    "#,
                )
                .expect("schema 2");
        }

        let store = Store::open(&path).expect("open migrates");
        assert_eq!(store.setting("favorites.model").expect("read"), None);
        store
            .set_setting("favorites.model", &serde_json::json!(["a"]))
            .expect("write");
        assert_eq!(
            store.setting("favorites.model").expect("read"),
            Some(serde_json::json!(["a"]))
        );

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn attachments_copy_once_link_to_a_turn_and_survive_a_trim() {
        let directory = std::env::temp_dir().join(format!("circulo-attach-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("temp dir");
        let source = directory.join("shot.txt");
        std::fs::write(&source, "hello").expect("source");

        let store = Store::open(&directory.join("circulo.db")).expect("store");
        let first = store.put_attachment(&source).expect("first");
        let second = store.put_attachment(&source).expect("second");

        assert_eq!(first.mime, "text/plain");
        assert_eq!(std::fs::read_to_string(&first.path).expect("blob"), "hello");
        assert_ne!(first.id, second.id, "each pick is its own row");
        assert_eq!(first.path, second.path, "same bytes, one blob");

        store.start_session("s1", None, "demo").expect("session");
        store.start_turn("s1", 1, "look").expect("turn");
        let summary = AttachmentSummary {
            id: first.id.clone(),
            name: first.name.clone(),
            mime: first.mime.clone(),
            size: first.size,
        };
        store
            .link_turn_attachments("s1", 1, std::slice::from_ref(&summary))
            .expect("link");

        let turns = store.transcript("s1").expect("transcript");
        assert_eq!(turns[0].attachments, vec![summary]);

        store.trim_turns("s1", None).expect("trim");
        assert!(store.transcript("s1").expect("empty").is_empty());
        assert!(
            store.turn_attachments("s1", 1).expect("links").is_empty(),
            "the turn link went with the turn"
        );
        assert!(first.path.exists(), "the blob stays until a GC lands");

        let _ = std::fs::remove_dir_all(&directory);
    }
}
