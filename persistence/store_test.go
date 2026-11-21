package persistence

import (
	"path/filepath"
	"testing"
	"time"
)

func TestRoundTrip(t *testing.T) {
	s, err := Open(filepath.Join(t.TempDir(), "circulo.db"))
	if err != nil {
		t.Fatal(err)
	}
	defer s.Close()
	now := time.UnixMilli(1000)
	if err := s.UpsertProject(Project{ID: "p1", Path: "/work", Name: "work", Created: now}); err != nil {
		t.Fatal(err)
	}
	if err := s.UpsertSession(Session{ID: "ses_1", ProjectID: "p1", Title: "work", Created: now}); err != nil {
		t.Fatal(err)
	}
	if err := s.SaveMessage(Message{ID: "m1", SessionID: "ses_1", Role: "assistant", Text: "hel", Done: false}); err != nil {
		t.Fatal(err)
	}
	if err := s.SaveMessage(Message{ID: "m1", SessionID: "ses_1", Role: "assistant", Text: "hello", Done: true}); err != nil {
		t.Fatal(err)
	}
	got, err := s.Message("m1")
	if err != nil {
		t.Fatal(err)
	}
	if got.Text != "hello" || !got.Done {
		t.Fatalf("%#v", got)
	}
	projects, err := s.ListProjects()
	if err != nil || len(projects) != 1 || projects[0].Name != "work" {
		t.Fatalf("%v %v", projects, err)
	}
}
