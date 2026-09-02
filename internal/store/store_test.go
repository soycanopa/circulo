package store

import (
	"os"
	"path/filepath"
	"testing"
)

func TestLoadMissingFileIsEmpty(t *testing.T) {
	dir := t.TempDir()
	s := New(filepath.Join(dir, "settings.json"))
	set, err := s.Load()
	if err != nil {
		t.Fatal(err)
	}
	if len(set.Projects) != 0 {
		t.Errorf("projects = %v, want empty", set.Projects)
	}
}

func TestSaveLoadRoundTrip(t *testing.T) {
	dir := t.TempDir()
	s := New(filepath.Join(dir, "nested", "settings.json"))
	set := Settings{Projects: []Project{
		{ID: "p_1", Path: "/tmp/x", Mode: "managed"},
		{ID: "p_2", Path: "/tmp/y", Mode: "attach", URL: "http://127.0.0.1:4096"},
	}}
	if err := s.Save(set); err != nil {
		t.Fatal(err)
	}
	got, err := s.Load()
	if err != nil {
		t.Fatal(err)
	}
	if len(got.Projects) != 2 || got.Projects[1].URL != "http://127.0.0.1:4096" {
		t.Errorf("round trip = %+v", got)
	}
}

func TestSaveIsAtomicToExistingFile(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "settings.json")
	s := New(path)
	if err := s.Save(Settings{Projects: []Project{{ID: "a"}}}); err != nil {
		t.Fatal(err)
	}
	if err := s.Save(Settings{Projects: []Project{{ID: "b"}}}); err != nil {
		t.Fatal(err)
	}
	got, _ := s.Load()
	if len(got.Projects) != 1 || got.Projects[0].ID != "b" {
		t.Errorf("second save = %+v", got)
	}
	if _, err := os.Stat(path + ".tmp"); !os.IsNotExist(err) {
		t.Error("tmp file left behind")
	}
}

func TestCorruptFileIsError(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "settings.json")
	_ = os.WriteFile(path, []byte("{not json"), 0o644)
	if _, err := New(path).Load(); err == nil {
		t.Fatal("expected error for corrupt settings")
	}
}

func TestProjectIDStable(t *testing.T) {
	a := ProjectID("/tmp/x")
	b := ProjectID("/tmp/x")
	c := ProjectID("/tmp/y")
	if a != b {
		t.Error("same path must produce same id")
	}
	if a == c {
		t.Error("different paths must produce different ids")
	}
}
