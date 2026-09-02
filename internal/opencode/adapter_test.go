package opencode

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"net/http/httptest"
	"os/exec"
	"sync"
	"testing"
	"time"

	"circulogo/internal/agent/protocol"
)

// fakeOC is a minimal OpenCode server: serves /global/health, a scripted SSE
// /event stream, and records the commands the adapter issues.
type fakeOC struct {
	t         *testing.T
	mu        sync.Mutex
	requests  []string
	stream    chan string
	url       string
	closeOnce sync.Once
}

func newFakeOC(t *testing.T) *fakeOC {
	return &fakeOC{t: t, stream: make(chan string, 64)}
}

func (f *fakeOC) handler() http.Handler {
	mux := http.NewServeMux()
	mux.HandleFunc("GET /global/health", func(w http.ResponseWriter, r *http.Request) {
		_ = json.NewEncoder(w).Encode(HealthResponse{Healthy: true, Version: "test"})
	})
	mux.HandleFunc("GET /session", func(w http.ResponseWriter, r *http.Request) {
		f.record(r)
		_ = json.NewEncoder(w).Encode([]Session{{ID: "ses_a", Title: "one"}, {ID: "ses_b", Title: "two"}})
	})
	mux.HandleFunc("POST /session", func(w http.ResponseWriter, r *http.Request) {
		f.record(r)
		_ = json.NewEncoder(w).Encode(Session{ID: "ses_new", Title: "made"})
	})
	mux.HandleFunc("GET /session/ses_h/message", func(w http.ResponseWriter, r *http.Request) {
		f.record(r)
		_ = json.NewEncoder(w).Encode([]MessagesPage{{
			Info: messageUser("ses_h"),
			Parts: []Part{
				{ID: "prt_u", Type: "text", Text: "hello", SessionID: "ses_h", MessageID: "msg_u"},
				{ID: "prt_drop", Type: "snapshot"},
			},
		}, {
			Info: messageAssistant("ses_h"),
			Parts: []Part{
				{ID: "prt_ss", Type: "step-start", SessionID: "ses_h", MessageID: "msg_a"},
				{ID: "prt_t", Type: "text", Text: "OK", SessionID: "ses_h", MessageID: "msg_a"},
			},
		}})
	})
	mux.HandleFunc("POST /session/ses_new/prompt_async", func(w http.ResponseWriter, r *http.Request) {
		f.record(r)
		w.WriteHeader(http.StatusNoContent)
	})
	mux.HandleFunc("POST /session/ses_new/abort", func(w http.ResponseWriter, r *http.Request) {
		f.record(r)
		_ = json.NewEncoder(w).Encode(true)
	})
	mux.HandleFunc("POST /session/ses_new/permissions/per_1", func(w http.ResponseWriter, r *http.Request) {
		f.record(r)
		_ = json.NewEncoder(w).Encode(true)
	})
	mux.HandleFunc("GET /event", func(w http.ResponseWriter, r *http.Request) {
		f.record(r)
		w.Header().Set("Content-Type", "text/event-stream")
		flusher := w.(http.Flusher)
		_, _ = fmt.Fprint(w, "data: {\"type\":\"server.connected\",\"properties\":{}}\n\n")
		flusher.Flush()
		for {
			select {
			case <-r.Context().Done():
				return
			case line := <-f.stream:
				_, _ = fmt.Fprintf(w, "data: %s\n\n", line)
				flusher.Flush()
			}
		}
	})
	return mux
}

func (f *fakeOC) record(r *http.Request) {
	f.mu.Lock()
	defer f.mu.Unlock()
	f.requests = append(f.requests, r.Method+" "+r.URL.Path)
}

func (f *fakeOC) hasRequest(method, path string) bool {
	f.mu.Lock()
	defer f.mu.Unlock()
	for _, r := range f.requests {
		if r == method+" "+path {
			return true
		}
	}
	return false
}

func (f *fakeOC) pushEvent(typ string, props map[string]any) {
	f.t.Helper()
	raw, _ := json.Marshal(map[string]any{"type": typ, "properties": props})
	select {
	case f.stream <- string(raw):
	case <-time.After(2 * time.Second):
		f.t.Fatal("fake server stream backlog full")
	}
}

func startFake(t *testing.T) *fakeOC {
	t.Helper()
	f := newFakeOC(t)
	srv := httptest.NewServer(f.handler())
	f.url = srv.URL
	t.Cleanup(srv.Close)
	return f
}

// drain collects adapter events until the predicate says stop or timeout.
func drain(t *testing.T, ch <-chan protocol.Envelope, stop func(protocol.Envelope) bool) []protocol.Envelope {
	t.Helper()
	var out []protocol.Envelope
	deadline := time.After(5 * time.Second)
	for {
		select {
		case e := <-ch:
			out = append(out, e)
			if stop(e) {
				return out
			}
		case <-deadline:
			t.Fatalf("timed out collecting events; got %v", envelopeTypes(out))
		}
	}
}

func payloadAs(t *testing.T, e protocol.Envelope, dst any) {
	t.Helper()
	if err := json.Unmarshal(e.Payload, dst); err != nil {
		t.Fatalf("unmarshal %s payload: %v", e.Type, err)
	}
}

func messageUser(sessionID string) Message {
	var m Message
	m.ID, m.SessionID, m.Role = "msg_u", sessionID, "user"
	m.Time.Created = 1788350998816
	return m
}

func messageAssistant(sessionID string) Message {
	var m Message
	m.ID, m.SessionID, m.Role = "msg_a", sessionID, "assistant"
	m.Time.Created = 1788350999000
	m.Time.Completed = 1788351001000
	m.Cost = 0.01
	m.Finish = "stop"
	return m
}

func TestAdapter_Attach_LifecycleAndTurn(t *testing.T) {
	f := startFake(t)
	a := NewAdapter(AdapterConfig{ProjectID: "p1", Mode: ModeAttach, URL: f.url})

	ctx := context.Background()
	if err := a.Start(ctx); err != nil {
		t.Fatal(err)
	}

	// First event must be adapter.status running.
	evts := drain(t, a.Events(), func(e protocol.Envelope) bool {
		return e.Type == protocol.EventAdapterStatus
	})
	var st protocol.AdapterStatus
	payloadAs(t, evts[len(evts)-1], &st)
	if st.State != protocol.AdapterRunning || st.ProjectID != "p1" {
		t.Errorf("status = %+v", st)
	}

	// Stream a turn through the fake server.
	f.pushEvent("session.updated", map[string]any{"sessionID": "ses_new", "info": map[string]any{
		"id": "ses_new", "title": "made", "time": map[string]any{"created": 1, "updated": 2}}})
	f.pushEvent("message.part.updated", map[string]any{"part": map[string]any{
		"id": "prt_1", "type": "text", "text": "OK", "messageID": "msg_1", "sessionID": "ses_new"},
		"delta": "OK"})
	f.pushEvent("message.part.updated", map[string]any{"part": map[string]any{
		"id": "prt_2", "type": "future.part", "messageID": "msg_1", "sessionID": "ses_new"}})

	evts = drain(t, a.Events(), func(e protocol.Envelope) bool {
		return e.Type == protocol.EventPartUpdated
	})
	var pu protocol.PartUpdated
	payloadAs(t, evts[len(evts)-1], &pu)
	if pu.Part.Text != "OK" || pu.Part.Type != protocol.PartText || pu.SessionID != "ses_new" {
		t.Errorf("part = %+v", pu)
	}
	// The delta hint follows the full part.
	select {
	case e := <-a.Events():
		if e.Type != protocol.EventPartDelta {
			t.Fatalf("expected part.delta, got %s", e.Type)
		}
		var pd protocol.PartDelta
		payloadAs(t, e, &pd)
		if pd.Delta != "OK" || pd.PartID != "prt_1" {
			t.Errorf("delta = %+v", pd)
		}
	case <-time.After(2 * time.Second):
		t.Fatal("no delta event")
	}

	// Commands proxy through the client.
	if err := a.Prompt(ctx, "ses_new", protocol.PromptRequest{Text: "hi", Agent: "build"}); err != nil {
		t.Fatal(err)
	}
	if !f.hasRequest("POST", "/session/ses_new/prompt_async") {
		t.Error("prompt_async not called")
	}
	if err := a.Abort(ctx, "ses_new"); err != nil {
		t.Fatal(err)
	}
	if !f.hasRequest("POST", "/session/ses_new/abort") {
		t.Error("abort not called")
	}
	if err := a.ReplyPermission(ctx, "ses_new", "per_1", protocol.PermissionOnce); err != nil {
		t.Fatal(err)
	}
	if !f.hasRequest("POST", "/session/ses_new/permissions/per_1") {
		t.Error("permission reply not called")
	}

	ss, err := a.Sessions(ctx)
	if err != nil || len(ss) != 2 || ss[0].Title != "one" {
		t.Errorf("sessions = %+v err=%v", ss, err)
	}

	// Hydration drops non-renderable parts.
	msgs, err := a.Messages(ctx, "ses_h", 0)
	if err != nil {
		t.Fatal(err)
	}
	if len(msgs) != 2 {
		t.Fatalf("messages = %d, want 2", len(msgs))
	}
	var types []string
	for _, p := range msgs[1].Parts {
		types = append(types, p.Type)
	}
	if len(types) != 2 || types[0] != protocol.PartStepStart || types[1] != protocol.PartText {
		t.Errorf("hydrated parts = %v (snapshot must be dropped)", types)
	}

	if err := a.Stop(ctx); err != nil {
		t.Fatal(err)
	}
	// Events closes shortly after Stop.
	select {
	case _, ok := <-a.Events():
		if ok {
			t.Fatal("expected closed events channel")
		}
	case <-time.After(2 * time.Second):
		t.Fatal("events not closed after Stop")
	}
	// Stop is idempotent.
	if err := a.Stop(ctx); err != nil {
		t.Fatal(err)
	}
}

func TestAdapter_AttachFailsWhenUnreachable(t *testing.T) {
	a := NewAdapter(AdapterConfig{ProjectID: "p", Mode: ModeAttach, URL: "http://127.0.0.1:1"})
	if err := a.Start(context.Background()); err == nil {
		t.Fatal("expected health failure")
	}
}

// The OpenCode session store is global: GET /session and the event stream
// carry sessions from every directory. The adapter must scope them to the
// server's root (GET /path → directory).
func TestAdapter_SessionScoping(t *testing.T) {
	root := "/private/tmp/proj"

	mux := http.NewServeMux()
	mux.HandleFunc("GET /global/health", func(w http.ResponseWriter, r *http.Request) {
		_ = json.NewEncoder(w).Encode(HealthResponse{Healthy: true})
	})
	mux.HandleFunc("GET /path", func(w http.ResponseWriter, r *http.Request) {
		_ = json.NewEncoder(w).Encode(PathResponse{Directory: root})
	})
	mux.HandleFunc("GET /session", func(w http.ResponseWriter, r *http.Request) {
		_ = json.NewEncoder(w).Encode([]Session{
			{ID: "ses_in", Title: "in project", Directory: root},
			{ID: "ses_tmp", Title: "via symlink", Directory: "/tmp/proj"}, // resolved == root
			{ID: "ses_out", Title: "other project", Directory: "/Users/x/other"},
		})
	})
	srv := httptest.NewServer(mux)
	defer srv.Close()

	a := NewAdapter(AdapterConfig{ProjectID: "p", Mode: ModeAttach, URL: srv.URL})
	if err := a.Start(context.Background()); err != nil {
		t.Fatal(err)
	}
	drain(t, a.Events(), func(e protocol.Envelope) bool { return e.Type == protocol.EventAdapterStatus })

	ss, err := a.Sessions(context.Background())
	if err != nil {
		t.Fatal(err)
	}
	// Contract: the server reports resolved directories, so only ses_in
	// matches the root; a raw "/tmp/proj" is a different path by contract.
	if len(ss) != 1 || ss[0].ID != "ses_in" {
		t.Errorf("scoped sessions = %+v, want ses_in only", ss)
	}

	// A live session.updated from a foreign directory must not reach Events().
	foreign := map[string]any{"sessionID": "ses_out", "info": map[string]any{
		"id": "ses_out", "title": "other", "directory": "/Users/x/other",
		"time": map[string]any{"created": 1, "updated": 1}}}
	raw, _ := json.Marshal(map[string]any{"type": "session.updated", "properties": foreign})
	// Push through a mini fake stream: reuse the adapter's client is not
	// possible here, so assert the filter function directly for the event path.
	env, err := DecodeEvent(Frame(raw))
	if err != nil {
		t.Fatal(err)
	}
	if !a.foreignSessionEvent(env) {
		t.Error("foreign session.updated not detected")
	}
	ours := map[string]any{"sessionID": "ses_in", "info": map[string]any{
		"id": "ses_in", "title": "in", "directory": root,
		"time": map[string]any{"created": 1, "updated": 1}}}
	raw, _ = json.Marshal(map[string]any{"type": "session.updated", "properties": ours})
	env, _ = DecodeEvent(Frame(raw))
	if a.foreignSessionEvent(env) {
		t.Error("in-scope session.updated wrongly flagged foreign")
	}

	_ = a.Stop(context.Background())
}

func TestAdapter_AttachRequiresURL(t *testing.T) {
	a := NewAdapter(AdapterConfig{ProjectID: "p", Mode: ModeAttach})
	if err := a.Start(context.Background()); err == nil {
		t.Fatal("expected error for missing URL")
	}
}

func TestAdapter_UnknownMode(t *testing.T) {
	a := NewAdapter(AdapterConfig{ProjectID: "p", Mode: "acp"})
	if err := a.Start(context.Background()); err == nil {
		t.Fatal("expected error for unknown mode")
	}
}

// Integration: full managed lifecycle against the real `opencode` binary.
// Skipped when the binary is absent or -short is passed.
func TestAdapter_Managed_Integration_Opencode(t *testing.T) {
	if testing.Short() {
		t.Skip("short mode")
	}
	if _, err := exec.LookPath("opencode"); err != nil {
		t.Skip("opencode not installed")
	}
	a := NewAdapter(AdapterConfig{ProjectID: "p", Mode: ModeManaged, Dir: t.TempDir()})
	if err := a.Start(context.Background()); err != nil {
		t.Fatal(err)
	}
	// First event: adapter running (after health + /event connected).
	evts := drain(t, a.Events(), func(e protocol.Envelope) bool {
		return e.Type == protocol.EventAdapterStatus
	})
	var st protocol.AdapterStatus
	payloadAs(t, evts[len(evts)-1], &st)
	if st.State != protocol.AdapterRunning {
		t.Errorf("state = %+v", st)
	}
	// The managed server really serves sessions.
	if _, err := a.Sessions(context.Background()); err != nil {
		t.Fatalf("sessions against real server: %v", err)
	}
	if err := a.Stop(context.Background()); err != nil {
		t.Fatal(err)
	}
}
