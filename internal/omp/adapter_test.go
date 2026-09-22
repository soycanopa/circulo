package omp

import (
	"bufio"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"

	"circulogo/internal/agent/protocol"
)

// TestFakeOmpChild is the scripted `omp --mode rpc` child used by the adapter
// tests (the classic TestHelperProcess pattern): the test binary re-executes
// itself under CIRCULOGO_FAKE_OMP_CHILD=1. The child speaks enough of the
// protocol to drive the adapter, replaying real fixture frames after prompt.
func TestFakeOmpChild(t *testing.T) {
	if os.Getenv("CIRCULOGO_FAKE_OMP_CHILD") != "1" {
		t.Skip("helper process")
	}
	in := bufio.NewScanner(os.Stdin)
	in.Buffer(make([]byte, 64*1024), 4<<20)
	out := bufio.NewWriter(os.Stdout)
	defer out.Flush()

	// Handshake.
	fmt.Fprintln(out, fakeReadyLine)
	out.Flush()

	// Every inbound command line lands here so tests can assert exactly what
	// the adapter sent (prompt vs steer, set_thinking_level, ...).
	var stdinLog *os.File
	if logPath := os.Getenv("CIRCULOGO_FAKE_STDIN_LOG"); logPath != "" {
		stdinLog, _ = os.OpenFile(logPath, os.O_CREATE|os.O_APPEND|os.O_WRONLY, 0o644)
	}

	// Current session identity; switch_session/new_session update it.
	sessionID := "sess0"
	answer := func(format string, args ...any) {
		fmt.Fprintf(out, format+"\n", args...)
		out.Flush()
	}

	for in.Scan() {
		if stdinLog != nil {
			stdinLog.Write(append(in.Bytes(), '\n'))
		}
		var cmd struct {
			ID          string `json:"id"`
			Type        string `json:"type"`
			Message     string `json:"message"`
			Name        string `json:"name"`
			SessionPath string `json:"sessionPath"`
		}
		if json.Unmarshal(in.Bytes(), &cmd) != nil {
			continue
		}
		switch cmd.Type {
		case "negotiate_protocol":
			answer(`{"id":%q,"type":"response","command":"negotiate_protocol","success":true,"data":{"protocolVersion":2}}`, cmd.ID)
		case "get_state":
			streaming := os.Getenv("CIRCULOGO_FAKE_STREAMING") == "1"
			answer(`{"id":%q,"type":"response","command":"get_state","success":true,"data":{"sessionId":%q,"isStreaming":%v,"sessionName":"sess"}}`, cmd.ID, sessionID, streaming)
		case "get_available_models":
			// Real captured catalog response (2 models): unwrap the fixture
			// frame's data payload and re-wrap it under this request's id.
			raw, err := os.ReadFile(os.Getenv("CIRCULOGO_FAKE_MODELS"))
			if err != nil {
				os.Exit(3)
			}
			var frame struct {
				Data json.RawMessage `json:"data"`
			}
			var data []byte
			for _, l := range strings.Split(string(raw), "\n") {
				l = strings.TrimSpace(l)
				if l == "" || l[0] == '#' {
					continue
				}
				if json.Unmarshal([]byte(l), &frame) == nil && len(frame.Data) > 0 {
					data = frame.Data
				}
			}
			answer(`{"id":%q,"type":"response","command":"get_available_models","success":true,"data":%s}`, cmd.ID, data)
		case "new_session":
			sessionID = "sess-new"
			writeChildSessionFile(sessionID)
			answer(`{"id":%q,"type":"response","command":"new_session","success":true,"data":{"cancelled":false}}`, cmd.ID)
		case "switch_session":
			base := filepath.Base(cmd.SessionPath)
			base = strings.TrimSuffix(base, ".jsonl")
			if i := strings.LastIndexByte(base, '_'); i >= 0 {
				sessionID = base[i+1:]
			}
			answer(`{"id":%q,"type":"response","command":"switch_session","success":true,"data":{"cancelled":false}}`, cmd.ID)
		case "set_session_name":
			answer(`{"id":%q,"type":"response","command":"set_session_name","success":true}`, cmd.ID)
		case "set_thinking_level":
			answer(`{"id":%q,"type":"response","command":"set_thinking_level","success":true}`, cmd.ID)
		case "steer", "prompt":
			answer(`{"id":%q,"type":"response","command":%q,"success":true}`, cmd.ID, cmd.Type)
			// Replay real captured turn frames (events only).
			replayFixtureEvents(out, os.Getenv("CIRCULOGO_FAKE_FIXTURE"))
			out.Flush()
		case "abort":
			answer(`{"id":%q,"type":"response","command":"abort","success":true}`, cmd.ID)
		default:
			if cmd.ID != "" {
				answer(`{"id":%q,"type":"response","command":%q,"success":true}`, cmd.ID, cmd.Type)
			}
		}
	}
	os.Exit(0)
}

func replayFixtureEvents(out io.Writer, path string) {
	raw, err := os.ReadFile(path)
	if err != nil {
		return
	}
	for _, line := range strings.Split(string(raw), "\n") {
		line = strings.TrimSpace(line)
		if line == "" || line[0] == '#' {
			continue
		}
		var e struct {
			Type string `json:"type"`
		}
		if json.Unmarshal([]byte(line), &e) != nil {
			continue
		}
		switch e.Type {
		case "response", "ready":
			continue
		}
		fmt.Fprintln(out, line)
	}
}

// writeChildSessionFile mirrors omp: new_session immediately persists the
// session JSONL in the cwd bucket, so hosts can switch back to it later.
func writeChildSessionFile(id string) {
	home, err := os.UserHomeDir()
	if err != nil {
		return
	}
	cwd, err := os.Getwd()
	if err != nil {
		return
	}
	bucket, err := encodeCwdBucket(cwd)
	if err != nil {
		return
	}
	dir := filepath.Join(home, sessionRoot, bucket)
	if err := os.MkdirAll(dir, 0o755); err != nil {
		return
	}
	entry := `{"type":"session","version":3,"id":"` + id + `","timestamp":"2026-02-16T10:20:30.000Z","cwd":"` + cwd + `"}`
	_ = os.WriteFile(filepath.Join(dir, "0000_"+id+".jsonl"), []byte(entry+"\n"), 0o644)
}

// startFakeAdapter builds an adapter over the scripted child and a fake HOME.
func startFakeAdapter(t *testing.T) (*Adapter, <-chan protocol.Envelope, string) {
	t.Helper()
	fakeHome := t.TempDir()
	prev := fakeHomePath
	fakeHomePath = fakeHome
	t.Cleanup(func() { fakeHomePath = prev })
	t.Setenv("HOME", fakeHome)
	t.Setenv("CIRCULOGO_FAKE_OMP_CHILD", "1")
	fixtures, err := filepath.Abs("testdata")
	if err != nil {
		t.Fatal(err)
	}
	// The child's cwd is the project dir, so fixture paths must be absolute.
	t.Setenv("CIRCULOGO_FAKE_FIXTURE", filepath.Join(fixtures, "turn-tool.jsonl"))
	t.Setenv("CIRCULOGO_FAKE_MODELS", filepath.Join(fixtures, "models.jsonl"))
	stdinLog := filepath.Join(t.TempDir(), "stdin.log")
	t.Setenv("CIRCULOGO_FAKE_STDIN_LOG", stdinLog)

	projectDir := t.TempDir()
	// One persisted session for discovery/switch/delete tests. The bucket
	// must be computed the way the adapter does it.
	bucket, err := encodeCwdBucket(projectDir)
	if err != nil {
		t.Fatal(err)
	}
	writeSessionFileAt(t, fakeHome, bucket, "100_sess0.jsonl", "Persisted", "sess0", "hello there")

	a := NewAdapter(AdapterConfig{
		ProjectID: "p1",
		Dir:       projectDir,
		Command:   []string{os.Args[0], "-test.run=TestFakeOmpChild", "--"},
	})
	return a, a.Events(), stdinLog
}

// writeSessionFileAt is writeSessionFile with an explicit fake home root.
func writeSessionFileAt(t *testing.T, home, bucket, name, title, id, firstUser string) string {
	t.Helper()
	slot := `{"type":"title","title":"` + title + `"}`
	if len(slot) < 256 {
		slot += strings.Repeat(" ", 256-len(slot))
	}
	header := `{"type":"session","version":3,"id":"` + id + `","timestamp":"2026-02-16T10:20:30.000Z","cwd":"/work/x"}`
	content := slot + "\n" + header + "\n"
	if firstUser != "" {
		entry := map[string]any{
			"type": "message",
			"message": map[string]any{
				"role":    "user",
				"content": []map[string]any{{"type": "text", "text": firstUser}},
			},
		}
		raw, _ := json.Marshal(entry)
		content += string(raw) + "\n"
	}
	path := filepath.Join(home, sessionRoot, bucket, name)
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(path, []byte(content), 0o644); err != nil {
		t.Fatal(err)
	}
	return path
}

// collect drains events until the predicate matches or the deadline passes.
func collect(t *testing.T, events <-chan protocol.Envelope, d time.Duration, match func(protocol.Envelope) bool) []protocol.Envelope {
	t.Helper()
	var got []protocol.Envelope
	deadline := time.After(d)
	for {
		select {
		case env, ok := <-events:
			if !ok {
				return got
			}
			got = append(got, env)
			if match(env) {
				return got
			}
		case <-deadline:
			return got
		}
	}
}

func waitForStatus(t *testing.T, events <-chan protocol.Envelope, state string) {
	t.Helper()
	envs := collect(t, events, 5*time.Second, func(env protocol.Envelope) bool {
		if env.Type != protocol.EventAdapterStatus {
			return false
		}
		var s protocol.AdapterStatus
		json.Unmarshal(env.Payload, &s)
		return s.State == state
	})
	if len(envs) == 0 {
		t.Fatalf("adapter.status %s never arrived", state)
	}
}

func TestAdapterLifecycleAgainstFakeChild(t *testing.T) {
	a, events, _ := startFakeAdapter(t)
	ctx := context.Background()
	// A mid-test failure must not leak the child (it would hold the test
	// binary's pipes and hang `go test`); Stop is idempotent.
	t.Cleanup(func() { _ = a.Stop(context.Background()) })

	if err := a.Start(ctx); err != nil {
		t.Fatal(err)
	}
	waitForStatus(t, events, protocol.AdapterRunning)

	// Sessions from the fake bucket.
	sessions, err := a.Sessions(ctx)
	if err != nil || len(sessions) != 1 || sessions[0].ID != "sess0" {
		t.Fatalf("sessions = %+v err=%v", sessions, err)
	}

	// Create: the child reports sess-new; title applies.
	sess, err := a.CreateSession(ctx, "Fresh")
	if err != nil || sess.ID != "sess-new" || sess.Title != "Fresh" {
		t.Fatalf("create = %+v err=%v", sess, err)
	}

	// Rename the non-active persisted session: switch → rename → switch back.
	if err := a.RenameSession(ctx, "sess0", "Renamed"); err != nil {
		t.Fatal(err)
	}
	renamed := collect(t, events, 3*time.Second, func(env protocol.Envelope) bool {
		if env.Type != protocol.EventSessionUpdated {
			return false
		}
		var u protocol.SessionUpdated
		json.Unmarshal(env.Payload, &u)
		return u.Session.ID == "sess0" && u.Session.Title == "Renamed"
	})
	if len(renamed) == 0 {
		t.Fatal("renamed session.updated never emitted")
	}

	// Prompt replays the tool fixture: expect busy → tool part → idle. The
	// collection window must run to the turn's END (idle), or it would stop
	// at the first busy status and miss the rest of the turn.
	if err := a.Prompt(ctx, "sess-new", protocol.PromptRequest{Text: "run the thing"}); err != nil {
		t.Fatal(err)
	}
	envs := collect(t, events, 5*time.Second, func(env protocol.Envelope) bool {
		if env.Type != protocol.EventSessionStatus {
			return false
		}
		var s protocol.SessionStatus
		json.Unmarshal(env.Payload, &s)
		return s.Status == protocol.StatusIdle
	})
	var sawBusy, sawTool, sawIdle bool
	for _, env := range envs {
		switch env.Type {
		case protocol.EventSessionStatus:
			var s protocol.SessionStatus
			json.Unmarshal(env.Payload, &s)
			if s.Status == protocol.StatusBusy {
				sawBusy = true
			}
			if s.Status == protocol.StatusIdle {
				sawIdle = true
			}
		case protocol.EventPartUpdated:
			var u protocol.PartUpdated
			json.Unmarshal(env.Payload, &u)
			if u.Part.Type == protocol.PartTool && u.Part.State != nil {
				if u.Part.State.Status == protocol.ToolCompleted && strings.Contains(u.Part.State.Output, "circulo-tool-smoke-7") {
					sawTool = true
				}
				if len(u.Part.State.Input) == 0 {
					t.Fatal("tool part lost its input in a full-replacement update")
				}
			}
		}
	}
	if !sawBusy || !sawTool || !sawIdle {
		t.Fatalf("turn incomplete: busy=%v tool=%v idle=%v", sawBusy, sawTool, sawIdle)
	}

	// Meta surfaces the captured catalog entries.
	meta, err := a.Meta(ctx)
	if err != nil {
		t.Fatal(err)
	}
	if len(meta.Models) != 2 || meta.Models[0].ID != "claude-4-sonnet" || meta.Models[0].Provider != "cursor" {
		t.Fatalf("meta models = %+v", meta.Models)
	}
	if !meta.Models[0].Reasoning || len(meta.Models[0].Variants) == 0 {
		t.Fatalf("reasoning model variants missing: %+v", meta.Models[0])
	}

	// Permission round-trip is explicitly unsupported for omp.
	if err := a.ReplyPermission(ctx, "sess-new", "perm", "once"); err == nil {
		t.Fatal("expected ReplyPermission to report unsupported")
	}

	// Forms: unknown id errors; the adapter registers kinds on FormUpdated.
	if err := a.ReplyForm(ctx, "sess-new", "missing", map[string]any{"value": "x"}); err == nil {
		t.Fatal("expected unknown form error")
	}

	// Delete refuses the active session.
	if err := a.DeleteSession(ctx, "sess-new"); err == nil {
		t.Fatal("expected active-session delete refusal")
	}
	// ...and removes a persisted one.
	if err := a.DeleteSession(ctx, "sess0"); err != nil {
		t.Fatal(err)
	}
	envs = collect(t, events, 2*time.Second, func(env protocol.Envelope) bool {
		return env.Type == protocol.EventSessionRemoved
	})
	if len(envs) == 0 {
		t.Fatal("session.removed never emitted")
	}

	// Stop is idempotent and closes the event channel.
	if err := a.Stop(ctx); err != nil {
		t.Fatal(err)
	}
	if err := a.Stop(ctx); err != nil {
		t.Fatal(err)
	}
	select {
	case _, ok := <-events:
		if ok {
			t.Fatal("events should be closed after Stop")
		}
	case <-time.After(2 * time.Second):
		t.Fatal("events channel never closed")
	}
}

func TestAdapterPromptQueuesAsSteerWhileStreaming(t *testing.T) {
	a, events, stdinLog := startFakeAdapter(t)
	t.Setenv("CIRCULOGO_FAKE_STREAMING", "1")
	ctx := context.Background()
	t.Cleanup(func() { _ = a.Stop(context.Background()) })

	if err := a.Start(ctx); err != nil {
		t.Fatal(err)
	}
	waitForStatus(t, events, protocol.AdapterRunning)

	// Streaming child on its current session: the prompt must go out as
	// steer with a thinking level.
	if err := a.Prompt(ctx, "sess0", protocol.PromptRequest{
		Text: "queued while busy", Variant: "high",
	}); err != nil {
		t.Fatal(err)
	}
	logged, err := os.ReadFile(stdinLog)
	if err != nil {
		t.Fatal(err)
	}
	log := string(logged)
	if !strings.Contains(log, `"type":"steer"`) {
		t.Fatalf("expected steer command in stdin log, got:\n%s", log)
	}
	if !strings.Contains(log, `"type":"set_thinking_level"`) || !strings.Contains(log, `"level":"high"`) {
		t.Fatalf("expected thinking level command, got:\n%s", log)
	}
	// The idle path sends prompt, not steer (covered by the lifecycle test's
	// log-free run; assert here for the same child for completeness).
	if strings.Contains(log, `"type":"prompt"`) {
		t.Fatalf("streaming child must not receive prompt, got:\n%s", log)
	}
	if err := a.Stop(ctx); err != nil {
		t.Fatal(err)
	}
}
