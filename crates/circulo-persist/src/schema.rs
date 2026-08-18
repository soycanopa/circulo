pub const MIGRATION_V1: &str = "
CREATE TABLE IF NOT EXISTS schema_migrations (
    version INTEGER PRIMARY KEY
);

CREATE TABLE IF NOT EXISTS projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    color TEXT,
    status TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    project_id TEXT,
    title TEXT NOT NULL,
    agent TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    last_message_at TEXT,
    first_send_at TEXT,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS messages (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    role TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at TEXT NOT NULL,
    is_streaming INTEGER NOT NULL,
    position INTEGER NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS message_parts (
    message_id TEXT NOT NULL,
    position INTEGER NOT NULL,
    payload TEXT NOT NULL,
    PRIMARY KEY (message_id, position),
    FOREIGN KEY (message_id) REFERENCES messages(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS preferences (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
";

/// V2: agent-side session binding for OpenCode (see change adapter-opencode).
pub const MIGRATION_V2: &str = "ALTER TABLE sessions ADD COLUMN opencode_session_id TEXT;";

/// V3: per-session composer chip settings.
pub const MIGRATION_V3: &str = "
ALTER TABLE sessions ADD COLUMN composer_model_id TEXT;
ALTER TABLE sessions ADD COLUMN composer_permission_mode TEXT;
ALTER TABLE sessions ADD COLUMN composer_interaction_mode TEXT;
";

/// V4: per-session model variant (OpenCode reasoning effort).
pub const MIGRATION_V4: &str = "ALTER TABLE sessions ADD COLUMN composer_model_variant TEXT;";
