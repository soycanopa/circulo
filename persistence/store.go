// Package persistence owns the core's SQLite database.
// Only the core process imports it.
package persistence

import (
	"database/sql"
	"os"
	"path/filepath"
	"time"

	_ "modernc.org/sqlite"
)

// Project is a folder the user added.
type Project struct {
	ID       string
	Path     string
	Name     string
	Provider string
	Created  time.Time
}

// Session is one provider conversation inside a project.
type Session struct {
	ID        string
	ProjectID string
	Title     string
	Provider  string
	Created   time.Time
}

// Checkpoint ties a Git revision to the message it was taken before.
type Checkpoint struct {
	ID        string
	SessionID string
	MessageID string
	Rev       string
	Created   time.Time
}

// Message is the text of one turn as stored.
type Message struct {
	ID        string
	SessionID string
	Role      string
	Text      string
	Done      bool
}

// Store is the local database.
type Store struct {
	db *sql.DB
}

// Open creates the database file and its tables.
func Open(path string) (*Store, error) {
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		return nil, err
	}
	db, err := sql.Open("sqlite", path)
	if err != nil {
		return nil, err
	}
	// One connection. Two writers on separate connections return SQLITE_BUSY.
	db.SetMaxOpenConns(1)
	s := &Store{db: db}
	if err := s.migrate(); err != nil {
		db.Close()
		return nil, err
	}
	return s, nil
}

func (s *Store) Close() error { return s.db.Close() }

func (s *Store) migrate() error {
	_, err := s.db.Exec(`
		PRAGMA foreign_keys = ON;
		PRAGMA busy_timeout = 5000;
		CREATE TABLE IF NOT EXISTS projects (
			id TEXT PRIMARY KEY,
			path TEXT NOT NULL UNIQUE,
			name TEXT NOT NULL,
			created_at INTEGER NOT NULL
		);
		CREATE TABLE IF NOT EXISTS sessions (
			id TEXT PRIMARY KEY,
			project_id TEXT NOT NULL REFERENCES projects(id),
			title TEXT NOT NULL,
			created_at INTEGER NOT NULL
		);
		CREATE TABLE IF NOT EXISTS messages (
			id TEXT PRIMARY KEY,
			session_id TEXT NOT NULL REFERENCES sessions(id),
			role TEXT NOT NULL,
			text TEXT NOT NULL,
			done INTEGER NOT NULL
		);
		CREATE TABLE IF NOT EXISTS checkpoints (
			id TEXT PRIMARY KEY,
			session_id TEXT NOT NULL REFERENCES sessions(id),
			message_id TEXT NOT NULL,
			rev TEXT NOT NULL,
			created_at INTEGER NOT NULL
		);
	`)
	if err != nil {
		return err
	}
	if err := s.addColumn("projects", "provider", "TEXT NOT NULL DEFAULT 'opencode'"); err != nil {
		return err
	}
	return s.addColumn("sessions", "provider", "TEXT NOT NULL DEFAULT 'opencode'")
}

func (s *Store) addColumn(table, column, decl string) error {
	rows, err := s.db.Query("PRAGMA table_info(" + table + ")")
	if err != nil {
		return err
	}
	defer rows.Close()
	for rows.Next() {
		var cid, notnull, pk int
		var name, typ string
		var dflt sql.NullString
		if err := rows.Scan(&cid, &name, &typ, &notnull, &dflt, &pk); err != nil {
			return err
		}
		if name == column {
			return nil
		}
	}
	if err := rows.Err(); err != nil {
		return err
	}
	_, err = s.db.Exec("ALTER TABLE " + table + " ADD COLUMN " + column + " " + decl)
	return err
}

// UpsertProject inserts or updates a project keyed by its path.
func (s *Store) UpsertProject(p Project) error {
	_, err := s.db.Exec(`
		INSERT INTO projects (id, path, name, provider, created_at) VALUES (?, ?, ?, ?, ?)
		ON CONFLICT(path) DO UPDATE SET name = excluded.name, provider = excluded.provider
	`, p.ID, p.Path, p.Name, providerOr(p.Provider), p.Created.UnixMilli())
	return err
}

// ListProjects returns projects oldest first.
func (s *Store) ListProjects() ([]Project, error) {
	rows, err := s.db.Query(`SELECT id, path, name, provider, created_at FROM projects ORDER BY created_at`)
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	var out []Project
	for rows.Next() {
		var p Project
		var ms int64
		if err := rows.Scan(&p.ID, &p.Path, &p.Name, &p.Provider, &ms); err != nil {
			return nil, err
		}
		p.Created = time.UnixMilli(ms)
		out = append(out, p)
	}
	return out, rows.Err()
}

// ProjectByPath returns the stored project for a folder.
func (s *Store) ProjectByPath(path string) (Project, error) {
	var p Project
	var ms int64
	err := s.db.QueryRow(`SELECT id, path, name, provider, created_at FROM projects WHERE path = ?`, path).
		Scan(&p.ID, &p.Path, &p.Name, &p.Provider, &ms)
	if err != nil {
		return Project{}, err
	}
	p.Created = time.UnixMilli(ms)
	return p, nil
}

// UpsertSession inserts or renames a session.
func (s *Store) UpsertSession(sess Session) error {
	_, err := s.db.Exec(`
		INSERT INTO sessions (id, project_id, title, provider, created_at) VALUES (?, ?, ?, ?, ?)
		ON CONFLICT(id) DO UPDATE SET title = excluded.title, provider = excluded.provider
	`, sess.ID, sess.ProjectID, sess.Title, providerOr(sess.Provider), sess.Created.UnixMilli())
	return err
}

// ListSessions returns sessions oldest first.
func (s *Store) ListSessions() ([]Session, error) {
	rows, err := s.db.Query(`SELECT id, project_id, title, provider, created_at FROM sessions ORDER BY created_at`)
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	var out []Session
	for rows.Next() {
		var sess Session
		var ms int64
		if err := rows.Scan(&sess.ID, &sess.ProjectID, &sess.Title, &sess.Provider, &ms); err != nil {
			return nil, err
		}
		sess.Created = time.UnixMilli(ms)
		out = append(out, sess)
	}
	return out, rows.Err()
}

// SessionByID returns one session.
func (s *Store) SessionByID(id string) (Session, error) {
	var sess Session
	var ms int64
	err := s.db.QueryRow(`SELECT id, project_id, title, provider, created_at FROM sessions WHERE id = ?`, id).
		Scan(&sess.ID, &sess.ProjectID, &sess.Title, &sess.Provider, &ms)
	if err != nil {
		return Session{}, err
	}
	sess.Created = time.UnixMilli(ms)
	return sess, nil
}

// SaveMessage inserts or replaces a message body.
func (s *Store) SaveMessage(m Message) error {
	done := 0
	if m.Done {
		done = 1
	}
	_, err := s.db.Exec(`
		INSERT INTO messages (id, session_id, role, text, done) VALUES (?, ?, ?, ?, ?)
		ON CONFLICT(id) DO UPDATE SET text = excluded.text, done = excluded.done
	`, m.ID, m.SessionID, m.Role, m.Text, done)
	return err
}

// Message returns one stored message.
func (s *Store) Message(id string) (Message, error) {
	var m Message
	var done int
	err := s.db.QueryRow(`SELECT id, session_id, role, text, done FROM messages WHERE id = ?`, id).
		Scan(&m.ID, &m.SessionID, &m.Role, &m.Text, &done)
	if err != nil {
		return Message{}, err
	}
	m.Done = done == 1
	return m, nil
}

// ListMessages returns the turns of one session in insertion order.
func (s *Store) ListMessages(sessionID string) ([]Message, error) {
	rows, err := s.db.Query(`SELECT id, session_id, role, text, done FROM messages WHERE session_id = ? ORDER BY rowid`, sessionID)
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	var out []Message
	for rows.Next() {
		var m Message
		var done int
		if err := rows.Scan(&m.ID, &m.SessionID, &m.Role, &m.Text, &done); err != nil {
			return nil, err
		}
		m.Done = done == 1
		out = append(out, m)
	}
	return out, rows.Err()
}

func providerOr(v string) string {
	if v == "" {
		return "opencode"
	}
	return v
}

// SaveCheckpoint stores the revision taken before messageID.
func (s *Store) SaveCheckpoint(c Checkpoint) error {
	_, err := s.db.Exec(`
		INSERT INTO checkpoints (id, session_id, message_id, rev, created_at) VALUES (?, ?, ?, ?, ?)
		ON CONFLICT(id) DO UPDATE SET rev = excluded.rev
	`, c.ID, c.SessionID, c.MessageID, c.Rev, c.Created.UnixMilli())
	return err
}

// LatestCheckpoint is the newest revision for a session.
func (s *Store) LatestCheckpoint(sessionID string) (Checkpoint, error) {
	var c Checkpoint
	var ms int64
	err := s.db.QueryRow(`
		SELECT id, session_id, message_id, rev, created_at FROM checkpoints
		WHERE session_id = ? ORDER BY created_at DESC LIMIT 1
	`, sessionID).Scan(&c.ID, &c.SessionID, &c.MessageID, &c.Rev, &ms)
	if err != nil {
		return Checkpoint{}, err
	}
	c.Created = time.UnixMilli(ms)
	return c, nil
}

// CheckpointByMessage returns the checkpoint stored for one message.
func (s *Store) CheckpointByMessage(messageID string) (Checkpoint, error) {
	var c Checkpoint
	var ms int64
	err := s.db.QueryRow(`
		SELECT id, session_id, message_id, rev, created_at FROM checkpoints WHERE message_id = ?
	`, messageID).Scan(&c.ID, &c.SessionID, &c.MessageID, &c.Rev, &ms)
	if err != nil {
		return Checkpoint{}, err
	}
	c.Created = time.UnixMilli(ms)
	return c, nil
}

// DeleteFromMessage drops that message, every later turn, and their checkpoints.
func (s *Store) DeleteFromMessage(messageID string) error {
	if _, err := s.db.Exec(`
		DELETE FROM checkpoints WHERE session_id = (SELECT session_id FROM messages WHERE id = ?)
			AND created_at >= (SELECT created_at FROM checkpoints WHERE message_id = ?)
	`, messageID, messageID); err != nil {
		return err
	}
	_, err := s.db.Exec(`
		DELETE FROM messages WHERE session_id = (SELECT session_id FROM messages WHERE id = ?)
			AND rowid >= (SELECT rowid FROM messages WHERE id = ?)
	`, messageID, messageID)
	return err
}
