package relay_test

import (
	"encoding/json"
	"fmt"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
	"time"

	"circulogo/internal/agent/protocol"
)

// The relay's checkpoint routes answer for the project, not the adapter: the
// refs live in the project's own repository, so a backend that is down must
// not take rewind with it.

func gitProject(t *testing.T) string {
	t.Helper()
	dir := t.TempDir()
	for _, args := range [][]string{
		{"init", "-b", "main"},
		{"config", "user.email", "t@example.com"},
		{"config", "user.name", "test"},
	} {
		cmd := exec.Command("git", args...)
		cmd.Dir = dir
		if out, err := cmd.CombinedOutput(); err != nil {
			t.Fatalf("git %v: %v\n%s", args, err, out)
		}
	}
	if err := os.WriteFile(filepath.Join(dir, "README.md"), []byte("one\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	for _, args := range [][]string{{"add", "."}, {"commit", "-m", "initial"}} {
		cmd := exec.Command("git", args...)
		cmd.Dir = dir
		if out, err := cmd.CombinedOutput(); err != nil {
			t.Fatalf("git %v: %v\n%s", args, err, out)
		}
	}
	return dir
}

// waitFor polls until cond holds, so the tests do not depend on how quickly
// the orchestrator's turn bookkeeping has run.
func waitFor(t *testing.T, d time.Duration, cond func() bool, msg string) {
	t.Helper()
	deadline := time.Now().Add(d)
	for time.Now().Before(deadline) {
		if cond() {
			return
		}
		time.Sleep(20 * time.Millisecond)
	}
	t.Fatal(msg)
}

func getJSON(t *testing.T, url string, into any) int {
	t.Helper()
	resp, err := http.Get(url)
	if err != nil {
		t.Fatal(err)
	}
	defer resp.Body.Close()
	if into != nil {
		_ = json.NewDecoder(resp.Body).Decode(into)
	}
	return resp.StatusCode
}

func postJSON(t *testing.T, url string, into any) int {
	t.Helper()
	resp, err := http.Post(url, "application/json", strings.NewReader("{}"))
	if err != nil {
		t.Fatal(err)
	}
	defer resp.Body.Close()
	if into != nil {
		_ = json.NewDecoder(resp.Body).Decode(into)
	}
	return resp.StatusCode
}

func TestCheckpoints_ReportedUnavailableOutsideGit(t *testing.T) {
	srv, _ := newServer(t)
	pv := addProject(t, srv, t.TempDir())
	id := pv["id"].(string)

	var view struct {
		Available bool `json:"available"`
	}
	if code := getJSON(t, srv.URL+"/agent/projects/"+id+"/checkpoints?session=ses_1", &view); code != http.StatusOK {
		t.Fatalf("status = %d, want 200", code)
	}
	if view.Available {
		t.Error("a plain directory must report checkpoints as unavailable")
	}
}

func TestCheckpoints_ListsStoredTurns(t *testing.T) {
	srv, ff := newServer(t)
	dir := gitProject(t)
	pv := addProject(t, srv, dir)
	id := pv["id"].(string)

	// Drive a turn through the adapter so the orchestrator's turn
	// bookkeeping sees the same status events a real backend would send.
	a := adapterFor(t, ff, dir)
	push := func(status string) {
		env, err := protocol.NewEnvelope(protocol.EventSessionStatus, protocol.SessionStatus{
			ProjectID: id, SessionID: "ses_1", Status: status,
		})
		if err != nil {
			t.Fatal(err)
		}
		a.events <- env
	}
	push(protocol.StatusBusy)
	if err := os.WriteFile(filepath.Join(dir, "agent.txt"), []byte("work\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	push(protocol.StatusIdle)
	waitFor(t, 3*time.Second, func() bool {
		var view struct {
			Checkpoints []struct {
				Kind string `json:"kind"`
			} `json:"checkpoints"`
		}
		getJSON(t, srv.URL+"/agent/projects/"+id+"/checkpoints?session=ses_1", &view)
		return len(view.Checkpoints) == 2
	}, "checkpoints never appeared over REST")

	var view struct {
		Available   bool `json:"available"`
		Dirty       bool `json:"dirty"`
		Checkpoints []struct {
			Turn int    `json:"turn"`
			Kind string `json:"kind"`
			Ref  string `json:"ref"`
		} `json:"checkpoints"`
	}
	if code := getJSON(t, srv.URL+"/agent/projects/"+id+"/checkpoints?session=ses_1", &view); code != http.StatusOK {
		t.Fatalf("status = %d", code)
	}
	if !view.Available {
		t.Error("a real git project should report checkpoints as available")
	}
	if len(view.Checkpoints) != 2 {
		t.Fatalf("got %d checkpoints, want 2", len(view.Checkpoints))
	}
	if view.Checkpoints[0].Kind != "start" || view.Checkpoints[1].Kind != "end" {
		t.Errorf("kinds = %s/%s, want start/end", view.Checkpoints[0].Kind, view.Checkpoints[1].Kind)
	}
	if !view.Dirty {
		t.Error("the agent's uncommitted file means the tree is dirty")
	}
}

func TestRewindPreview_ShowsTheDiff(t *testing.T) {
	srv, ff := newServer(t)
	dir := gitProject(t)
	pv := addProject(t, srv, dir)
	id := pv["id"].(string)
	a := adapterFor(t, ff, dir)
	push := func(status string) {
		env, _ := protocol.NewEnvelope(protocol.EventSessionStatus, protocol.SessionStatus{
			ProjectID: id, SessionID: "ses_1", Status: status,
		})
		a.events <- env
	}
	count := func() int {
		var v struct {
			Checkpoints []any `json:"checkpoints"`
		}
		getJSON(t, srv.URL+"/agent/projects/"+id+"/checkpoints?session=ses_1", &v)
		return len(v.Checkpoints)
	}

	// The fan-in is asynchronous, so the turn has to be observed to have
	// started before the agent's edit lands — otherwise the file would be
	// captured as part of the pre-turn state and there would be nothing to
	// rewind.
	push(protocol.StatusBusy)
	waitFor(t, 3*time.Second, func() bool { return count() == 1 }, "turn never started")
	if err := os.WriteFile(filepath.Join(dir, "agent.txt"), []byte("work\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	push(protocol.StatusIdle)
	waitFor(t, 3*time.Second, func() bool { return count() == 2 }, "turn never ended")

	var preview struct {
		Turn     int    `json:"turn"`
		Diff     string `json:"diff"`
		WillLose bool   `json:"willLose"`
	}
	url := fmt.Sprintf("%s/agent/projects/%s/checkpoints/rewind-preview?session=ses_1&turn=1", srv.URL, id)
	if code := getJSON(t, url, &preview); code != http.StatusOK {
		t.Fatalf("preview status = %d", code)
	}
	if !strings.Contains(preview.Diff, "agent.txt") {
		t.Errorf("preview must describe the file the turn created:\n%s", preview.Diff)
	}
	if !preview.WillLose {
		t.Error("uncommitted work means a rewind loses something")
	}
}

func TestRewind_RequiresAnExplicitTurn(t *testing.T) {
	srv, _ := newServer(t)
	pv := addProject(t, srv, gitProject(t))
	id := pv["id"].(string)
	for _, q := range []string{"?session=ses_1", "?session=ses_1&turn=0", "?session=ses_1&turn=abc", "?session=ses_1&turn=-3"} {
		var out map[string]string
		if code := getJSON(t, srv.URL+"/agent/projects/"+id+"/checkpoints/rewind-preview"+q, &out); code != http.StatusBadRequest {
			t.Errorf("turn=%q status = %d, want 400", q, code)
		}
	}
}

// Rewind is destructive, so an unknown turn must be refused rather than
// quietly rewinding to the newest one.
func TestRewind_RefusesUnknownTurn(t *testing.T) {
	srv, _ := newServer(t)
	pv := addProject(t, srv, gitProject(t))
	id := pv["id"].(string)
	var out map[string]string
	url := fmt.Sprintf("%s/agent/projects/%s/checkpoints/rewind?session=ses_1&turn=99", srv.URL, id)
	if code := postJSON(t, url, &out); code == http.StatusOK {
		t.Error("rewinding to a turn with no checkpoint should fail")
	}
}

func TestRewind_UnknownProject(t *testing.T) {
	srv, _ := newServer(t)
	var out map[string]string
	if code := getJSON(t, srv.URL+"/agent/projects/nope/checkpoints?session=ses_1", &out); code == http.StatusOK {
		t.Error("expected an error for an unknown project")
	}
}

// Commands arrive collapsed and ordered: a same-name collision must not reach
// the picker twice, and the winner is decided by scope rather than by
// whichever entry the backend happened to list first.
func TestMeta_ResolvesCommandCollisions(t *testing.T) {
	srv, ff := newServer(t)
	dir := gitProject(t)
	pv := addProject(t, srv, dir)
	id := pv["id"].(string)
	a := adapterFor(t, ff, dir)
	a.commands = []protocol.CommandInfo{
		{Name: "review", Scope: protocol.ScopeBuiltin},
		{Name: "review", Scope: protocol.ScopeProject},
		{Name: "deploy", Scope: protocol.ScopeSkill},
	}

	var meta struct {
		Commands []protocol.CommandInfo `json:"commands"`
	}
	if code := getJSON(t, fmt.Sprintf("%s/agent/projects/%s/meta", srv.URL, id), &meta); code != http.StatusOK {
		t.Fatalf("meta status = %d", code)
	}
	if len(meta.Commands) != 2 {
		t.Fatalf("got %d commands, want the collision collapsed: %+v", len(meta.Commands), meta.Commands)
	}
	var review *protocol.CommandInfo
	for i := range meta.Commands {
		if meta.Commands[i].Name == "review" {
			review = &meta.Commands[i]
		}
	}
	if review == nil {
		t.Fatalf("review missing: %+v", meta.Commands)
	}
	if review.Scope != protocol.ScopeProject {
		t.Errorf("review resolved to %q, want the project command to shadow the builtin", review.Scope)
	}
	// Display order is by scope rank, and the surviving review is now a
	// project command (rank 1) while deploy is a skill (rank 3) — so review
	// lists first even though it won a collision.
	if meta.Commands[0].Name != "review" || meta.Commands[1].Name != "deploy" {
		t.Errorf("display order = %+v, want review then deploy", meta.Commands)
	}
}
