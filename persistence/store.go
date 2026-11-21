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
	ID      string
	Path    string
	Name    string
	Created time.Time
}

// Session is one provider conversation inside a project.
type Session struct {
	ID        string
	ProjectID string
	Title     string
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
	`)
	return err
}

// UpsertProject inserts or updates a project keyed by its path.
func (s *Store) UpsertProject(p Project) error {
	_, err := s.db.Exec(`
		INSERT INTO projects (id, path, name, created_at) VALUES (?, ?, ?, ?)
		ON CONFLICT(path) DO UPDATE SET name = excluded.name
	`, p.ID, p.Path, p.Name, p.Created.UnixMilli())
	return err
}

// ListProjects returns projects oldest first.
func (s *Store) ListProjects() ([]Project, error) {
	rows, err := s.db.Query(`SELECT id, path, name, created_at FROM projects ORDER BY created_at`)
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	var out []Project
	for rows.Next() {
		var p Project
		var ms int64
		if err := rows.Scan(&p.ID, &p.Path, &p.Name, &ms); err != nil {
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
	err := s.db.QueryRow(`SELECT id, path, name, created_at FROM projects WHERE path = ?`, path).
		Scan(&p.ID, &p.Path, &p.Name, &ms)
	if err != nil {
		return Project{}, err
	}
	p.Created = time.UnixMilli(ms)
	return p, nil
}

// UpsertSession inserts or renames a session.
func (s *Store) UpsertSession(sess Session) error {
	_, err := s.db.Exec(`
		INSERT INTO sessions (id, project_id, title, created_at) VALUES (?, ?, ?, ?)
		ON CONFLICT(id) DO UPDATE SET title = excluded.title
	`, sess.ID, sess.ProjectID, sess.Title, sess.Created.UnixMilli())
	return err
}

// ListSessions returns sessions oldest first.
func (s *Store) ListSessions() ([]Session, error) {
	rows, err := s.db.Query(`SELECT id, project_id, title, created_at FROM sessions ORDER BY created_at`)
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	var out []Session
	for rows.Next() {
		var sess Session
		var ms int64
		if err := rows.Scan(&sess.ID, &sess.ProjectID, &sess.Title, &ms); err != nil {
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
	err := s.db.QueryRow(`SELECT id, project_id, title, created_at FROM sessions WHERE id = ?`, id).
		Scan(&sess.ID, &sess.ProjectID, &sess.Title, &ms)
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
