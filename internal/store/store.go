// Package store persists Circulo's own settings (projects, UI prefs) as a
// single JSON file. Chat history is deliberately NOT stored here: it lives in
// each agent server's own storage (docs/trd.md §7).
package store

import (
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"sync"
)

// Project is one configured project folder.
type Project struct {
	ID   string `json:"id"`
	Path string `json:"path"`
	Mode string `json:"mode"` // managed|attach (opencode.Mode* values; string here to avoid an import cycle)
	URL  string `json:"url,omitempty"`
	// Provider selects the agent backend ("opencode" | "omp"). Empty means
	// "opencode": settings written before providers existed stay valid.
	Provider string `json:"provider,omitempty"`
	// AccessMode is the provider's access mode (protocol.Access* values;
	// string here to avoid an import cycle). Empty = provider default.
	AccessMode string `json:"accessMode,omitempty"`
}

// Settings is the whole settings file.
type Settings struct {
	Projects []Project      `json:"projects"`
	UI       map[string]any `json:"ui,omitempty"`
}

// Store is a file-backed settings store. Safe for concurrent use.
type Store struct {
	mu   sync.Mutex
	path string
}

// New targets path (e.g. ~/Library/Application Support/circulo/settings.json).
func New(path string) *Store { return &Store{path: path} }

const appDir = "circulo"

// legacyAppDir is where releases before the rename stored their settings. It
// still holds real user data — project rows the user added by hand — so the
// path change migrates rather than abandoning it.
const legacyAppDir = "circulogo"

// DefaultPath returns the OS config location, migrating a pre-rename settings
// file into the new directory the first time it is found.
//
// A renamed config directory would otherwise look like a fresh install: the
// user's projects would silently vanish, with no error to explain it.
func DefaultPath() (string, error) {
	dir, err := os.UserConfigDir()
	if err != nil {
		return "", err
	}
	return defaultPathIn(dir)
}

// defaultPathIn is the testable core. It takes the config directory explicitly
// so a test can never reach the real one: an earlier version of this migration
// was exercised through DefaultPath, and on a machine where
// os.UserConfigDir ignores XDG_CONFIG_HOME the test performed the real
// migration on the developer's own settings file.
func defaultPathIn(configDir string) (string, error) {
	newPath := filepath.Join(configDir, appDir, "settings.json")
	legacy := filepath.Join(configDir, legacyAppDir, "settings.json")
	if _, err := os.Stat(newPath); err == nil {
		return newPath, nil // already on the new path
	}
	if _, err := os.Stat(legacy); err != nil {
		return newPath, nil // nothing to migrate
	}
	if err := os.MkdirAll(filepath.Dir(newPath), 0o755); err != nil {
		// Not fatal: fall back to the old location so the user keeps their
		// projects, which matters far more than the directory name.
		return legacy, nil
	}
	if err := os.Rename(legacy, newPath); err != nil {
		return legacy, nil
	}
	return newPath, nil
}

// ProjectID derives a stable id from the project path so re-adding the same
// folder never duplicates rows across restarts.
func ProjectID(path string) string {
	sum := sha256.Sum256([]byte(path))
	return "p_" + hex.EncodeToString(sum[:4])
}

// Load reads settings; a missing file yields empty settings, not an error.
func (s *Store) Load() (Settings, error) {
	s.mu.Lock()
	defer s.mu.Unlock()
	raw, err := os.ReadFile(s.path)
	if errors.Is(err, os.ErrNotExist) {
		return Settings{}, nil
	}
	if err != nil {
		return Settings{}, err
	}
	var out Settings
	if err := json.Unmarshal(raw, &out); err != nil {
		return Settings{}, fmt.Errorf("store: corrupt settings %s: %w", s.path, err)
	}
	return out, nil
}

// Save writes settings atomically (tmp + rename) so a crash cannot truncate it.
func (s *Store) Save(set Settings) error {
	s.mu.Lock()
	defer s.mu.Unlock()
	if err := os.MkdirAll(filepath.Dir(s.path), 0o755); err != nil {
		return err
	}
	raw, err := json.MarshalIndent(set, "", "  ")
	if err != nil {
		return err
	}
	tmp := s.path + ".tmp"
	if err := os.WriteFile(tmp, raw, 0o644); err != nil {
		return err
	}
	return os.Rename(tmp, s.path)
}
