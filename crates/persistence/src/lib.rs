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

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

use circulo_protocol::Part;
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::Value;

/// Where the database lives when `CIRCULO_HOME` is not set.
const HOME_DIRECTORY: &str = ".circulo";
const DATABASE_FILE: &str = "circulo.db";

/// Schema version, tracked in SQLite's own `user_version`.
///
/// 1 created the tables. 2 added `sessions.provider_session`, the provider's
/// own key for a session, so a restart can hand it back and the agent resumes
/// where it left off.
const SCHEMA_VERSION: i64 = 2;

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

/// One turn of a session, ready to be drawn again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnRecord {
    pub turn: u64,
    pub prompt: String,
    pub parts: Vec<Part>,
}

/// The handle the core holds. One connection behind a mutex: writes come from
/// turn threads, reads from the connection that answers a client.
pub struct Store {
    connection: Mutex<Connection>,
}

impl Store {
    /// Opens the database, creating its directory and schema when missing.
    pub fn open(path: &Path) -> Result<Self, StoreError> {
        if let Some(directory) = path.parent() {
            std::fs::create_dir_all(directory)?;
        }

        let store = Self {
            connection: Mutex::new(Connection::open(path)?),
        };
        store.migrate()?;
        Ok(store)
    }

    /// A throwaway database, for tests.
    pub fn in_memory() -> Result<Self, StoreError> {
        let store = Self {
            connection: Mutex::new(Connection::open_in_memory()?),
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

    /// Every turn of a session, in order, with its parts in order.
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
                    "INSERT INTO sessions (id, provider, created_at, updated_at)
                     VALUES ('s1', 'demo', 0, 0)",
                    [],
                )
                .expect("row");
        }

        let store = Store::open(&path).expect("open migrates");
        assert_eq!(store.sessions().expect("list")[0].id, "s1", "data survived");
        assert_eq!(store.provider_session("s1").expect("key"), None);
        store
            .set_provider_session("s1", Some("ses_acp_1"))
            .expect("set");

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn the_home_override_decides_the_path() {
        let path = home_path(Some("/tmp/circulo-home".into()), None).expect("path");
        assert_eq!(path, PathBuf::from("/tmp/circulo-home/circulo.db"));

        let path = home_path(None, Some("/home/me".into())).expect("path");
        assert_eq!(path, PathBuf::from("/home/me/.circulo/circulo.db"));

        assert!(home_path(None, None).is_err());
    }
}
