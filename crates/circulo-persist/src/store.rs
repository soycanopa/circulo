use std::path::{Path, PathBuf};

use circulo_core::{
    ComposerInteractionMode, ComposerPermissionMode, Message, MessagePart, MessageRole, Project,
    ProjectStatus, Session, UserPreferences, Uuid,
};
use rusqlite::{params, Connection, OptionalExtension};
use serde::de::{DeserializeOwned, Error as _};
use serde::Serialize;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::error::PersistError;
use crate::schema::{MIGRATION_V1, MIGRATION_V2, MIGRATION_V3, MIGRATION_V4};

const SESSION_COLUMNS: &str =
    "id, project_id, title, agent, status, created_at, updated_at, last_message_at, first_send_at, composer_model_id, composer_model_variant, composer_permission_mode, composer_interaction_mode";
const SESSION_COLUMNS_ALIASED: &str =
    "s.id, s.project_id, s.title, s.agent, s.status, s.created_at, s.updated_at, s.last_message_at, s.first_send_at, s.composer_model_id, s.composer_model_variant, s.composer_permission_mode, s.composer_interaction_mode";

pub fn default_db_path() -> Result<PathBuf, PersistError> {
    let home = std::env::var_os("HOME").ok_or(PersistError::InvalidHome)?;
    Ok(PathBuf::from(home).join("Library/Application Support/Circulo/circulo.sqlite"))
}

pub struct Store {
    conn: Connection,
}

impl Store {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, PersistError> {
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        Self::from_connection(conn)
    }

    pub fn open_in_memory() -> Result<Self, PersistError> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    pub fn open_default() -> Result<Self, PersistError> {
        Self::open(default_db_path()?)
    }

    fn from_connection(conn: Connection) -> Result<Self, PersistError> {
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.execute_batch(MIGRATION_V1)?;
        conn.execute(
            "INSERT OR IGNORE INTO schema_migrations (version) VALUES (1)",
            [],
        )?;
        if conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('sessions') WHERE name = 'opencode_session_id'",
            [],
            |row| row.get::<_, i64>(0),
        )? == 0
        {
            conn.execute_batch(MIGRATION_V2)?;
        }
        conn.execute(
            "INSERT OR IGNORE INTO schema_migrations (version) VALUES (2)",
            [],
        )?;
        if conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('sessions') WHERE name = 'composer_model_id'",
            [],
            |row| row.get::<_, i64>(0),
        )? == 0
        {
            conn.execute_batch(MIGRATION_V3)?;
        }
        conn.execute(
            "INSERT OR IGNORE INTO schema_migrations (version) VALUES (3)",
            [],
        )?;
        if conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('sessions') WHERE name = 'composer_model_variant'",
            [],
            |row| row.get::<_, i64>(0),
        )? == 0
        {
            conn.execute_batch(MIGRATION_V4)?;
        }
        conn.execute(
            "INSERT OR IGNORE INTO schema_migrations (version) VALUES (4)",
            [],
        )?;
        Ok(Self { conn })
    }

    pub fn create_project(&self, project: &Project) -> Result<(), PersistError> {
        self.conn.execute(
            "INSERT INTO projects (id, name, description, color, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                project.id.to_string(),
                project.name,
                project.description,
                project.color,
                enum_to_db(&project.status)?,
                format_time(project.created_at)?,
                format_time(project.updated_at)?,
            ],
        )?;
        Ok(())
    }

    pub fn list_active_projects(&self) -> Result<Vec<Project>, PersistError> {
        self.query_projects("SELECT id, name, description, color, status, created_at, updated_at FROM projects WHERE status = 'active' ORDER BY name")
    }

    pub fn list_archived_projects(&self) -> Result<Vec<Project>, PersistError> {
        self.query_projects("SELECT id, name, description, color, status, created_at, updated_at FROM projects WHERE status = 'archived' ORDER BY name")
    }

    pub fn archive_project(&self, id: Uuid) -> Result<(), PersistError> {
        self.set_project_status(id, ProjectStatus::Archived)
    }

    pub fn restore_project(&self, id: Uuid) -> Result<(), PersistError> {
        self.set_project_status(id, ProjectStatus::Active)
    }

    pub fn delete_project(&self, id: Uuid) -> Result<(), PersistError> {
        let n = self
            .conn
            .execute("DELETE FROM projects WHERE id = ?1", [id.to_string()])?;
        if n == 0 {
            return Err(PersistError::NotFound);
        }
        Ok(())
    }

    pub fn delete_session(&self, id: Uuid) -> Result<(), PersistError> {
        let n = self
            .conn
            .execute("DELETE FROM sessions WHERE id = ?1", [id.to_string()])?;
        if n == 0 {
            return Err(PersistError::NotFound);
        }
        Ok(())
    }

    pub fn get_project(&self, id: Uuid) -> Result<Option<Project>, PersistError> {
        self.conn
            .query_row(
                "SELECT id, name, description, color, status, created_at, updated_at
                 FROM projects WHERE id = ?1",
                [id.to_string()],
                map_project,
            )
            .optional()
            .map_err(PersistError::from)
    }

    pub fn update_project(&self, project: &Project) -> Result<(), PersistError> {
        let n = self.conn.execute(
            "UPDATE projects
             SET name = ?1, description = ?2, color = ?3, status = ?4, updated_at = ?5
             WHERE id = ?6",
            params![
                project.name,
                project.description,
                project.color,
                enum_to_db(&project.status)?,
                format_time(project.updated_at)?,
                project.id.to_string(),
            ],
        )?;
        if n == 0 {
            return Err(PersistError::NotFound);
        }
        Ok(())
    }

    pub fn create_session(&self, session: &Session) -> Result<(), PersistError> {
        self.conn.execute(
            "INSERT INTO sessions
                (id, project_id, title, agent, status, created_at, updated_at, last_message_at, first_send_at,
                 composer_model_id, composer_model_variant, composer_permission_mode, composer_interaction_mode)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                session.id.to_string(),
                session.project_id.map(|id| id.to_string()),
                session.title,
                enum_to_db(&session.agent)?,
                enum_to_db(&session.status)?,
                format_time(session.created_at)?,
                format_time(session.updated_at)?,
                session.last_message_at.map(format_time).transpose()?,
                session.first_send_at.map(format_time).transpose()?,
                session.composer_model_id,
                session.composer_model_variant,
                session
                    .composer_permission_mode
                    .map(|mode| enum_to_db(&mode))
                    .transpose()?,
                session
                    .composer_interaction_mode
                    .map(|mode| enum_to_db(&mode))
                    .transpose()?,
            ],
        )?;
        Ok(())
    }

    pub fn get_session(&self, id: Uuid) -> Result<Option<Session>, PersistError> {
        self.conn
            .query_row(
                &format!("SELECT {SESSION_COLUMNS} FROM sessions WHERE id = ?1"),
                [id.to_string()],
                map_session,
            )
            .optional()
            .map_err(PersistError::from)
    }

    pub fn list_visible_sessions(&self) -> Result<Vec<Session>, PersistError> {
        self.query_sessions(
            &format!(
                "SELECT {SESSION_COLUMNS_ALIASED}
             FROM sessions s
             LEFT JOIN projects p ON p.id = s.project_id
             WHERE s.status = 'active'
               AND (s.project_id IS NULL OR p.status = 'active')
             ORDER BY s.last_message_at IS NULL, s.last_message_at DESC, s.created_at DESC"
            ),
            [],
        )
    }

    pub fn list_unassigned_sessions(&self) -> Result<Vec<Session>, PersistError> {
        self.query_sessions(
            &format!(
                "SELECT {SESSION_COLUMNS}
             FROM sessions
             WHERE project_id IS NULL AND status = 'active'
             ORDER BY last_message_at IS NULL, last_message_at DESC, created_at DESC"
            ),
            [],
        )
    }

    pub fn list_sessions_for_project(
        &self,
        project_id: Uuid,
    ) -> Result<Vec<Session>, PersistError> {
        self.query_sessions(
            &format!(
                "SELECT {SESSION_COLUMNS_ALIASED}
             FROM sessions s
             INNER JOIN projects p ON p.id = s.project_id
             WHERE s.project_id = ?1 AND s.status = 'active' AND p.status = 'active'
             ORDER BY s.last_message_at IS NULL, s.last_message_at DESC, s.created_at DESC"
            ),
            [project_id.to_string()],
        )
    }

    pub fn search_sessions(&self, query: &str) -> Result<Vec<Session>, PersistError> {
        let pattern = like_contains(query);
        self.query_sessions(
            &format!(
                "SELECT {SESSION_COLUMNS_ALIASED}
             FROM sessions s
             LEFT JOIN projects p ON p.id = s.project_id
             WHERE s.status = 'active'
               AND (s.project_id IS NULL OR p.status = 'active')
               AND s.title LIKE ?1 ESCAPE '\\' COLLATE NOCASE
             ORDER BY s.last_message_at IS NULL, s.last_message_at DESC, s.created_at DESC"
            ),
            [pattern],
        )
    }

    pub fn update_session(&self, session: &Session) -> Result<(), PersistError> {
        let n = self.conn.execute(
            "UPDATE sessions
             SET project_id = ?1, title = ?2, status = ?3, updated_at = ?4,
                 last_message_at = ?5, first_send_at = ?6, composer_model_id = ?7,
                 composer_model_variant = ?8, composer_permission_mode = ?9,
                 composer_interaction_mode = ?10
             WHERE id = ?11",
            params![
                session.project_id.map(|id| id.to_string()),
                session.title,
                enum_to_db(&session.status)?,
                format_time(session.updated_at)?,
                session.last_message_at.map(format_time).transpose()?,
                session.first_send_at.map(format_time).transpose()?,
                session.composer_model_id,
                session.composer_model_variant,
                session
                    .composer_permission_mode
                    .map(|mode| enum_to_db(&mode))
                    .transpose()?,
                session
                    .composer_interaction_mode
                    .map(|mode| enum_to_db(&mode))
                    .transpose()?,
                session.id.to_string(),
            ],
        )?;
        if n == 0 {
            return Err(PersistError::NotFound);
        }
        Ok(())
    }

    pub fn assign_session_project(
        &self,
        session_id: Uuid,
        project_id: Option<Uuid>,
    ) -> Result<(), PersistError> {
        let mut session = self
            .get_session(session_id)?
            .ok_or(PersistError::NotFound)?;
        session
            .assign_project(project_id)
            .map_err(PersistError::Domain)?;
        self.conn.execute(
            "UPDATE sessions SET project_id = ?1, updated_at = ?2 WHERE id = ?3",
            params![
                session.project_id.map(|id| id.to_string()),
                format_time(session.updated_at)?,
                session.id.to_string(),
            ],
        )?;
        Ok(())
    }

    pub fn opencode_session_id(&self, id: Uuid) -> Result<Option<String>, PersistError> {
        let value = self
            .conn
            .query_row(
                "SELECT opencode_session_id FROM sessions WHERE id = ?1",
                [id.to_string()],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()?;
        Ok(value.flatten())
    }

    /// Binds a session to its OpenCode session. Write-once: storing the same id
    /// again is a no-op; storing a different id over an existing binding fails.
    pub fn bind_opencode_session(
        &self,
        id: Uuid,
        agent_session_id: &str,
    ) -> Result<(), PersistError> {
        let updated = self.conn.execute(
            "UPDATE sessions SET opencode_session_id = ?1 WHERE id = ?2 AND opencode_session_id IS NULL",
            params![agent_session_id, id.to_string()],
        )?;
        if updated > 0 {
            return Ok(());
        }
        match self.opencode_session_id(id)? {
            Some(existing) if existing == agent_session_id => Ok(()),
            Some(_) => Err(PersistError::AgentBindingLocked),
            None => Err(PersistError::NotFound),
        }
    }

    pub fn save_message(&self, message: &Message) -> Result<(), PersistError> {
        let tx = self.conn.unchecked_transaction()?;
        let next_pos: i64 = tx.query_row(
            "SELECT COALESCE(MAX(position), -1) + 1 FROM messages WHERE session_id = ?1",
            [message.session_id.to_string()],
            |row| row.get(0),
        )?;
        tx.execute(
            "INSERT INTO messages (id, session_id, role, status, created_at, is_streaming, position)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO UPDATE SET
                role = excluded.role,
                status = excluded.status,
                is_streaming = excluded.is_streaming",
            params![
                message.id.to_string(),
                message.session_id.to_string(),
                enum_to_db(&message.role)?,
                enum_to_db(&message.status)?,
                format_time(message.created_at)?,
                message.is_streaming as i64,
                next_pos,
            ],
        )?;
        tx.execute(
            "DELETE FROM message_parts WHERE message_id = ?1",
            [message.id.to_string()],
        )?;
        for (index, part) in message.parts.iter().enumerate() {
            tx.execute(
                "INSERT INTO message_parts (message_id, position, payload) VALUES (?1, ?2, ?3)",
                params![
                    message.id.to_string(),
                    index as i64,
                    serde_json::to_string(part)?,
                ],
            )?;
        }

        let first_send_at: Option<String> = tx
            .query_row(
                "SELECT first_send_at FROM sessions WHERE id = ?1",
                [message.session_id.to_string()],
                |row| row.get(0),
            )
            .optional()?
            .flatten();
        let stamp = format_time(message.created_at)?;
        if matches!(message.role, MessageRole::User) && first_send_at.is_none() {
            tx.execute(
                "UPDATE sessions SET first_send_at = ?1, last_message_at = ?1, updated_at = ?1 WHERE id = ?2",
                params![stamp, message.session_id.to_string()],
            )?;
        } else {
            tx.execute(
                "UPDATE sessions SET last_message_at = ?1, updated_at = ?1 WHERE id = ?2",
                params![stamp, message.session_id.to_string()],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn get_preferences(&self) -> Result<UserPreferences, PersistError> {
        let raw: Option<String> = self
            .conn
            .query_row(
                "SELECT value FROM preferences WHERE key = 'user'",
                [],
                |row| row.get(0),
            )
            .optional()?;
        match raw {
            Some(json) => serde_json::from_str(&json).map_err(PersistError::from),
            None => Ok(UserPreferences::default()),
        }
    }

    pub fn set_preferences(&self, preferences: &UserPreferences) -> Result<(), PersistError> {
        let json = serde_json::to_string(preferences)?;
        self.conn.execute(
            "INSERT INTO preferences (key, value) VALUES ('user', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [json],
        )?;
        Ok(())
    }

    pub fn list_messages(&self, session_id: Uuid) -> Result<Vec<Message>, PersistError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, session_id, role, status, created_at, is_streaming
             FROM messages WHERE session_id = ?1 ORDER BY position",
        )?;
        let rows = stmt.query_map([session_id.to_string()], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, i64>(5)?,
            ))
        })?;

        let mut messages = Vec::new();
        for row in rows {
            let (id, session, role, status, created_at, is_streaming) = row?;
            let mut part_stmt = self.conn.prepare(
                "SELECT payload FROM message_parts WHERE message_id = ?1 ORDER BY position",
            )?;
            let parts = part_stmt
                .query_map([&id], |row| row.get::<_, String>(0))?
                .map(|payload| {
                    let payload = payload?;
                    serde_json::from_str::<MessagePart>(&payload).map_err(|err| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(err),
                        )
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            messages.push(Message {
                id: parse_uuid(&id)?,
                session_id: parse_uuid(&session)?,
                role: enum_from_db(&role)?,
                parts,
                status: enum_from_db(&status)?,
                created_at: parse_time(&created_at)?,
                is_streaming: is_streaming != 0,
            });
        }
        Ok(messages)
    }

    #[cfg(test)]
    #[allow(dead_code)]
    pub(crate) fn conn_for_test(&self) -> &Connection {
        &self.conn
    }

    fn set_project_status(&self, id: Uuid, status: ProjectStatus) -> Result<(), PersistError> {
        let n = self.conn.execute(
            "UPDATE projects SET status = ?1, updated_at = ?2 WHERE id = ?3",
            params![
                enum_to_db(&status)?,
                format_time(OffsetDateTime::now_utc())?,
                id.to_string(),
            ],
        )?;
        if n == 0 {
            return Err(PersistError::NotFound);
        }
        Ok(())
    }

    fn query_projects(&self, sql: &str) -> Result<Vec<Project>, PersistError> {
        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map([], map_project)?;
        rows.map(|row| row.map_err(PersistError::from)).collect()
    }

    fn query_sessions<P: rusqlite::Params>(
        &self,
        sql: &str,
        params: P,
    ) -> Result<Vec<Session>, PersistError> {
        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map(params, map_session)?;
        rows.map(|row| row.map_err(PersistError::from)).collect()
    }
}

fn map_project(row: &rusqlite::Row<'_>) -> rusqlite::Result<Project> {
    Ok(Project {
        id: parse_uuid_sql(&row.get::<_, String>(0)?)?,
        name: row.get(1)?,
        description: row.get(2)?,
        color: row.get(3)?,
        status: enum_from_db_sql(&row.get::<_, String>(4)?)?,
        created_at: parse_time_sql(&row.get::<_, String>(5)?)?,
        updated_at: parse_time_sql(&row.get::<_, String>(6)?)?,
    })
}

fn map_session(row: &rusqlite::Row<'_>) -> rusqlite::Result<Session> {
    Ok(Session {
        id: parse_uuid_sql(&row.get::<_, String>(0)?)?,
        project_id: row
            .get::<_, Option<String>>(1)?
            .map(|id| parse_uuid_sql(&id))
            .transpose()?,
        title: row.get(2)?,
        agent: enum_from_db_sql(&row.get::<_, String>(3)?)?,
        status: enum_from_db_sql(&row.get::<_, String>(4)?)?,
        created_at: parse_time_sql(&row.get::<_, String>(5)?)?,
        updated_at: parse_time_sql(&row.get::<_, String>(6)?)?,
        last_message_at: row
            .get::<_, Option<String>>(7)?
            .map(|ts| parse_time_sql(&ts))
            .transpose()?,
        first_send_at: row
            .get::<_, Option<String>>(8)?
            .map(|ts| parse_time_sql(&ts))
            .transpose()?,
        composer_model_id: row.get(9)?,
        composer_model_variant: row.get(10)?,
        composer_permission_mode: row
            .get::<_, Option<String>>(11)?
            .map(|value| enum_from_db_sql(&value))
            .transpose()?,
        composer_interaction_mode: row
            .get::<_, Option<String>>(12)?
            .map(|value| enum_from_db_sql(&value))
            .transpose()?,
    })
}

fn enum_to_db<T: Serialize>(value: &T) -> Result<String, PersistError> {
    let json = serde_json::to_value(value)?;
    json.as_str().map(str::to_owned).ok_or_else(|| {
        PersistError::Serde(serde_json::Error::custom("enum must serialize as string"))
    })
}

fn enum_from_db<T: DeserializeOwned>(value: &str) -> Result<T, PersistError> {
    Ok(serde_json::from_value(serde_json::Value::String(
        value.to_owned(),
    ))?)
}

fn enum_from_db_sql<T: DeserializeOwned>(value: &str) -> rusqlite::Result<T> {
    enum_from_db(value).map_err(|err| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(err))
    })
}

fn format_time(value: OffsetDateTime) -> Result<String, PersistError> {
    value
        .format(&Rfc3339)
        .map_err(|err| PersistError::Serde(serde_json::Error::custom(err.to_string())))
}

fn parse_time(value: &str) -> Result<OffsetDateTime, PersistError> {
    Ok(OffsetDateTime::parse(value, &Rfc3339)?)
}

fn parse_time_sql(value: &str) -> rusqlite::Result<OffsetDateTime> {
    parse_time(value).map_err(|err| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(err))
    })
}

fn parse_uuid(value: &str) -> Result<Uuid, PersistError> {
    Uuid::parse_str(value)
        .map_err(|err| PersistError::Serde(serde_json::Error::custom(err.to_string())))
}

fn parse_uuid_sql(value: &str) -> rusqlite::Result<Uuid> {
    parse_uuid(value).map_err(|err| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(err))
    })
}

fn like_contains(query: &str) -> String {
    let escaped = query
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    format!("%{escaped}%")
}
