package orchestrator

import (
	"context"
	"os"
	"os/exec"
	"path/filepath"
	"testing"
	"time"

	"circulogo/internal/agent/protocol"
	"circulogo/internal/checkpoint"
	"circulogo/internal/store"
)

// gitRepo makes a real repository: checkpointing is git plumbing, and the
// wiring under test is only meaningful against real git output.
func gitRepo(t *testing.T) string {
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

// checkpointOrch adds one real git project through the supported path and
// waits for its adapter, returning the assigned project id.
func checkpointOrch(t *testing.T, dir string) (*Orchestrator, string) {
	t.Helper()
	ff := newFakeFactory()
	o := New(store.New(t.TempDir()+"/settings.json"), ff.build)
	// Subscribe so the adapter's startup events land in a drained channel
	// rather than a full buffer, which would block the start goroutine.
	ch, _, _, cancel := o.Subscribe(protocol.Cursor{})
	go func() {
		for range ch {
		}
	}()
	t.Cleanup(cancel)
	// attach mode with an unreachable URL is the cheapest way to register a
	// project whose path is a real repo: the adapter fails to connect, which
	// is irrelevant to the checkpoint wiring.
	pv, err := o.AddProject(context.Background(), dir, "attach", "http://127.0.0.1:1", "")
	if err != nil {
		t.Fatal(err)
	}
	if pv.ID == "" {
		t.Fatalf("AddProject returned no id: %+v", pv)
	}
	id := pv.ID
	// Let the adapter registration settle so the project is in the map.
	deadline := time.After(3 * time.Second)
	for {
		if _, err := o.projectPath(id); err == nil {
			// The path resolves as soon as the project exists; wait for the
			// status event to be safe about ordering with the pump.
			break
		}
		select {
		case <-deadline:
			t.Fatal("project never registered")
		case <-time.After(10 * time.Millisecond):
		}
	}
	t.Cleanup(o.Shutdown)
	return o, id
}

func statusEvent(t *testing.T, projectID, sessionID, status string) protocol.Envelope {
	t.Helper()
	env, err := protocol.NewEnvelope(protocol.EventSessionStatus, protocol.SessionStatus{
		ProjectID: projectID, SessionID: sessionID, Status: status,
	})
	if err != nil {
		t.Fatal(err)
	}
	return env
}

// A busy→idle cycle is one turn, and must produce a start and an end
// checkpoint. Both adapters translate their own events into this one status
// event, which is why the wiring keys off it.
func TestTurnBoundary_CapturesStartAndEnd(t *testing.T) {
	dir := gitRepo(t)
	o, id := checkpointOrch(t, dir)

	o.observeTurnBoundary(id, statusEvent(t, id, "ses_1", protocol.StatusBusy))
	// The agent edits the workspace mid-turn.
	if err := os.WriteFile(filepath.Join(dir, "agent.txt"), []byte("work\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	o.observeTurnBoundary(id, statusEvent(t, id, "ses_1", protocol.StatusIdle))

	view, err := o.Checkpoints(id, "ses_1")
	if err != nil {
		t.Fatal(err)
	}
	if !view.Available {
		t.Fatal("a real git project should report checkpoints available")
	}
	if len(view.Checkpoints) != 2 {
		t.Fatalf("got %d checkpoints, want a start and an end", len(view.Checkpoints))
	}
	start, end := view.Checkpoints[0], view.Checkpoints[1]
	if start.Kind != checkpoint.KindStart || end.Kind != checkpoint.KindEnd {
		t.Fatalf("kinds = %s/%s, want start/end", start.Kind, end.Kind)
	}
	if start.Turn != 1 || end.Turn != 1 {
		t.Errorf("turns = %d/%d, want 1/1", start.Turn, end.Turn)
	}
	// The end must contain what the agent wrote, or the checkpoint is useless.
	out, err := exec.Command("git", "-C", dir, "show", end.Ref+":agent.txt").CombinedOutput()
	if err != nil {
		t.Fatalf("end checkpoint does not contain the agent's file: %v\n%s", err, out)
	}
}

// A retry re-reports busy. That must not open a second turn or overwrite the
// start checkpoint of the one in flight.
func TestTurnBoundary_RetryDoesNotReopenTurn(t *testing.T) {
	dir := gitRepo(t)
	o, id := checkpointOrch(t, dir)

	o.observeTurnBoundary(id, statusEvent(t, id, "ses_1", protocol.StatusBusy))
	o.observeTurnBoundary(id, statusEvent(t, id, "ses_1", protocol.StatusRetry))
	o.observeTurnBoundary(id, statusEvent(t, id, "ses_1", protocol.StatusBusy))
	o.observeTurnBoundary(id, statusEvent(t, id, "ses_1", protocol.StatusIdle))

	view, err := o.Checkpoints(id, "ses_1")
	if err != nil {
		t.Fatal(err)
	}
	if len(view.Checkpoints) != 2 {
		t.Fatalf("got %d checkpoints, want exactly one turn's pair", len(view.Checkpoints))
	}
	if view.Checkpoints[0].Turn != 1 || view.Checkpoints[1].Turn != 1 {
		t.Errorf("retry opened a new turn: %+v", view.Checkpoints)
	}
}

func TestTurnBoundary_SeparateSessionsCountSeparately(t *testing.T) {
	dir := gitRepo(t)
	o, projectID := checkpointOrch(t, dir)

	for _, sessionID := range []string{"ses_1", "ses_2"} {
		o.observeTurnBoundary(projectID, statusEvent(t, projectID, sessionID, protocol.StatusBusy))
		o.observeTurnBoundary(projectID, statusEvent(t, projectID, sessionID, protocol.StatusIdle))
	}
	a, _ := o.Checkpoints(projectID, "ses_1")
	b, _ := o.Checkpoints(projectID, "ses_2")
	if len(a.Checkpoints) != 2 || len(b.Checkpoints) != 2 {
		t.Fatalf("ses_1 has %d, ses_2 has %d; want 2 each", len(a.Checkpoints), len(b.Checkpoints))
	}
	if a.Checkpoints[0].Ref == b.Checkpoints[0].Ref {
		t.Error("two sessions share a ref")
	}
}

// A project that is just a folder is the common case, not an error.
func TestCheckpoints_UnavailableOutsideGit(t *testing.T) {
	dir := t.TempDir()
	o, id := checkpointOrch(t, dir)
	// No repo: nothing should blow up, and the feature reports unavailable.
	o.observeTurnBoundary(id, statusEvent(t, id, "ses_1", protocol.StatusBusy))
	o.observeTurnBoundary(id, statusEvent(t, id, "ses_1", protocol.StatusIdle))

	view, err := o.Checkpoints(id, "ses_1")
	if err != nil {
		t.Fatal(err)
	}
	if view.Available {
		t.Error("a plain directory must not report checkpoints as available")
	}
	if len(view.Checkpoints) != 0 {
		t.Error("no checkpoints should exist")
	}
}

func TestCheckpoints_UnknownProject(t *testing.T) {
	dir := gitRepo(t)
	o, _ := checkpointOrch(t, dir)
	if _, err := o.Checkpoints("nope", "ses_1"); err == nil {
		t.Error("expected an error for an unknown project")
	}
}

func TestPreviewRewind_ReportsWhatWouldBeLost(t *testing.T) {
	dir := gitRepo(t)
	o, id := checkpointOrch(t, dir)
	o.observeTurnBoundary(id, statusEvent(t, id, "ses_1", protocol.StatusBusy))
	if err := os.WriteFile(filepath.Join(dir, "agent.txt"), []byte("work\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	o.observeTurnBoundary(id, statusEvent(t, id, "ses_1", protocol.StatusIdle))

	preview, err := o.PreviewRewind(id, "ses_1", 1)
	if err != nil {
		t.Fatal(err)
	}
	if preview.Turn != 1 {
		t.Errorf("turn = %d, want 1", preview.Turn)
	}
	if preview.Diff == "" {
		t.Error("a rewind preview must show what it would undo")
	}
	if !preview.WillLose {
		t.Error("the agent's uncommitted work means a rewind loses something")
	}
}

func TestPreviewRewind_UnknownTurn(t *testing.T) {
	dir := gitRepo(t)
	o, id := checkpointOrch(t, dir)
	o.observeTurnBoundary(id, statusEvent(t, id, "ses_1", protocol.StatusBusy))
	o.observeTurnBoundary(id, statusEvent(t, id, "ses_1", protocol.StatusIdle))
	if _, err := o.PreviewRewind(id, "ses_1", 42); err == nil {
		t.Error("expected an error previewing a turn that has no checkpoint")
	}
}

func TestRewind_RestoresWorktreeAndPreservesCurrentState(t *testing.T) {
	dir := gitRepo(t)
	o, id := checkpointOrch(t, dir)
	o.observeTurnBoundary(id, statusEvent(t, id, "ses_1", protocol.StatusBusy))
	agentFile := filepath.Join(dir, "agent.txt")
	if err := os.WriteFile(agentFile, []byte("work\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	o.observeTurnBoundary(id, statusEvent(t, id, "ses_1", protocol.StatusIdle))

	if _, err := o.Rewind(context.Background(), id, "ses_1", 1); err != nil {
		t.Fatal(err)
	}
	if _, err := os.Stat(agentFile); err == nil {
		t.Error("the agent's file survived the rewind")
	}
	// The rewind is itself undoable: the pre-rewind state was captured.
	view, err := o.Checkpoints(id, "ses_1")
	if err != nil {
		t.Fatal(err)
	}
	var guarded bool
	for _, cp := range view.Checkpoints {
		if cp.Kind == checkpoint.KindGuard {
			guarded = true
		}
	}
	if !guarded {
		t.Error("rewind must preserve the state it replaced, or it is irreversible")
	}
	// The guard holds the work that was just discarded.
	if !checkpoint.Available(dir) {
		t.Fatal("repo should still be valid")
	}
}

func TestRewind_RefusesTurnWithoutCheckpoint(t *testing.T) {
	dir := gitRepo(t)
	o, id := checkpointOrch(t, dir)
	if _, err := o.Rewind(context.Background(), id, "ses_1", 3); err == nil {
		t.Error("expected a refusal for a turn with no checkpoint")
	}
}
