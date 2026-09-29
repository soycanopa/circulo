package store

import (
	"os"
	"path/filepath"
	"testing"
)

// The app directory was renamed from "circulogo" to "circulo". That must not
// look like a fresh install: the settings file holds project rows the user
// added by hand, and losing them silently would be the worst outcome of a
// rename.

func TestDefaultPathIn_PrefersNewPathWhenBothExist(t *testing.T) {
	root := t.TempDir()
	mkSettings(t, filepath.Join(root, appDir, "settings.json"), `{"projects":[]}`)
	mkSettings(t, filepath.Join(root, legacyAppDir, "settings.json"), `{"projects":[{"id":"legacy"}]}`)

	got, err := defaultPathIn(root)
	if err != nil {
		t.Fatal(err)
	}
	want := filepath.Join(root, appDir, "settings.json")
	if got != want {
		t.Errorf("path = %q, want %q", got, want)
	}
	// The legacy file is left alone: it was not ours to delete.
	if _, err := os.Stat(filepath.Join(root, legacyAppDir, "settings.json")); err != nil {
		t.Error("the legacy settings file was removed even though the new one already existed")
	}
}

func TestDefaultPathIn_MigratesLegacyFile(t *testing.T) {
	root := t.TempDir()
	legacyPath := filepath.Join(root, legacyAppDir, "settings.json")
	mkSettings(t, legacyPath, `{"projects":[{"id":"p_keep"}]}`)

	got, err := defaultPathIn(root)
	if err != nil {
		t.Fatal(err)
	}
	want := filepath.Join(root, appDir, "settings.json")
	if got != want {
		t.Fatalf("path = %q, want %q", got, want)
	}
	// The data must have come with it.
	raw, err := os.ReadFile(want)
	if err != nil {
		t.Fatalf("migrated file missing: %v", err)
	}
	s, err := New(want).Load()
	if err != nil {
		t.Fatal(err)
	}
	if len(s.Projects) != 1 || s.Projects[0].ID != "p_keep" {
		t.Errorf("project lost in migration: %+v (raw %s)", s.Projects, raw)
	}
	if _, err := os.Stat(legacyPath); err == nil {
		t.Error("legacy file still present after migration")
	}
}

func TestDefaultPathIn_PlainFirstRun(t *testing.T) {
	root := t.TempDir()
	got, err := defaultPathIn(root)
	if err != nil {
		t.Fatal(err)
	}
	if want := filepath.Join(root, appDir, "settings.json"); got != want {
		t.Errorf("path = %q, want %q", got, want)
	}
	// A first run creates nothing eagerly.
	if _, err := os.Stat(filepath.Join(root, appDir)); err == nil {
		t.Error("DefaultPath created the directory on a first run")
	}
}

// If the move fails, keeping the old location is far better than losing the
// user's projects. A regular file where the new directory would go makes
// MkdirAll fail with ENOTDIR, which is deterministic across platforms.
func TestDefaultPathIn_MigrationFailureKeepsLegacy(t *testing.T) {
	root := t.TempDir()
	legacyPath := filepath.Join(root, legacyAppDir, "settings.json")
	mkSettings(t, legacyPath, `{"projects":[{"id":"p_keep"}]}`)
	// A plain file blocks the new directory.
	if err := os.WriteFile(filepath.Join(root, appDir), []byte("not a directory"), 0o644); err != nil {
		t.Fatal(err)
	}

	got, err := defaultPathIn(root)
	if err != nil {
		t.Fatal(err)
	}
	if got != legacyPath {
		t.Errorf("path = %q, want the legacy %q so the data stays reachable", got, legacyPath)
	}
	s, err := New(got).Load()
	if err != nil {
		t.Fatal(err)
	}
	if len(s.Projects) != 1 || s.Projects[0].ID != "p_keep" {
		t.Errorf("projects = %+v, want the legacy row still readable", s.Projects)
	}
}

func TestDefaultPathIn_MigrationIsIdempotent(t *testing.T) {
	root := t.TempDir()
	mkSettings(t, filepath.Join(root, legacyAppDir, "settings.json"), `{"projects":[{"id":"p1"}]}`)

	if _, err := defaultPathIn(root); err != nil {
		t.Fatal(err)
	}
	second, err := defaultPathIn(root)
	if err != nil {
		t.Fatal(err)
	}
	if second != filepath.Join(root, appDir, "settings.json") {
		t.Errorf("second call drifted: %q", second)
	}
	s, err := New(second).Load()
	if err != nil {
		t.Fatal(err)
	}
	if len(s.Projects) != 1 {
		t.Errorf("projects = %+v after a second call", s.Projects)
	}
}

// A test that reaches the real config directory is a test that can move the
// user's settings. This pins that the injectable core is what the suite uses.
func TestDefaultPathIn_NeverTouchesTheRealConfigDir(t *testing.T) {
	root := t.TempDir()
	// A legacy file in the temp dir, and none in the real one: if the suite
	// called DefaultPath() instead, the developer's own file would move.
	mkSettings(t, filepath.Join(root, legacyAppDir, "settings.json"), `{"projects":[]}`)
	if _, err := defaultPathIn(root); err != nil {
		t.Fatal(err)
	}
	real, err := os.UserConfigDir()
	if err != nil {
		t.Skip("no config dir on this platform")
	}
	if _, err := os.Stat(filepath.Join(real, appDir, "settings.json")); err != nil {
		t.Skip("no real settings on this machine")
	}
	legacy, err := os.Stat(filepath.Join(real, legacyAppDir, "settings.json"))
	if err == nil {
		t.Errorf("the real legacy settings file was moved by a test: %v", legacy)
	}
}

func mkSettings(t *testing.T, path, body string) {
	t.Helper()
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(path, []byte(body), 0o644); err != nil {
		t.Fatal(err)
	}
}
