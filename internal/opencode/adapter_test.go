package opencode

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/http/httptest"
	"os"
	"strings"
	"sync"
	"testing"
	"time"

	"circulogo/internal/agent/protocol"
)

// fakeOC is a minimal OpenCode v2 server: /api/info readiness, a scripted
// SSE /api/event stream, and request recording for the command endpoints.
type fakeOC struct {
	t         *testing.T
	mu        sync.Mutex
	requests  []string // "METHOD path"
	bodies    map[string]string
	stream    chan string
	url       string
	closeOnce sync.Once
}

func newFakeOC(t *testing.T) *fakeOC {
	return &fakeOC{t: t, stream: make(chan string, 64), bodies: map[string]string{}}
}

func (f *fakeOC) handler() http.Handler {
	mux := http.NewServeMux()
	mux.HandleFunc("GET /api/info", func(w http.ResponseWriter, r *http.Request) {
		_ = json.NewEncoder(w).Encode(V2ServerInfo{Version: "test", PID: 1})
	})
	mux.HandleFunc("GET /api/location", func(w http.ResponseWriter, r *http.Request) {
		_, _ = w.Write([]byte(`{"directory":"/private/tmp/proj","project":{"id":"prj","directory":"/private/tmp/proj"}}`))
	})
	mux.HandleFunc("GET /api/session", func(w http.ResponseWriter, r *http.Request) {
		f.record(r)
		_, _ = w.Write([]byte(`{"data":[
			{"id":"ses_a","projectID":"prj","title":"one","time":{"created":1,"updated":2},"location":{"directory":"/private/tmp/proj"}},
			{"id":"ses_b","projectID":"prj","title":"two","time":{"created":1,"updated":2},"location":{"directory":"/private/tmp/proj"}}]}`))
	})
	mux.HandleFunc("POST /api/session", func(w http.ResponseWriter, r *http.Request) {
		f.record(r)
		_, _ = w.Write([]byte(`{"data":{"id":"ses_new","projectID":"prj","title":"made","time":{"created":1,"updated":1},"location":{"directory":"/private/tmp/proj"}}}`))
	})
	mux.HandleFunc("PATCH /api/session/ses_new", func(w http.ResponseWriter, r *http.Request) {
		f.record(r)
		w.WriteHeader(http.StatusNoContent)
	})
	mux.HandleFunc("DELETE /api/session/ses_del", func(w http.ResponseWriter, r *http.Request) {
		f.record(r)
		w.WriteHeader(http.StatusNoContent)
	})
	mux.HandleFunc("GET /api/session/ses_h/message", func(w http.ResponseWriter, r *http.Request) {
		f.record(r)
		_, _ = w.Write([]byte(`{"data":[
			{"id":"msg_a","time":{"created":100,"completed":200},"type":"assistant","agent":"build",
			 "model":{"id":"m1","providerID":"p1"},"finish":"stop","cost":0.01,
			 "tokens":{"input":10,"output":2,"reasoning":0,"cache":{"read":0,"write":0}},
			 "content":[
			   {"type":"reasoning","text":"thinking"},
			   {"type":"tool","id":"call_1","name":"shell",
			    "state":{"status":"completed","input":{"command":"ls"},"output":"x"}},
			   {"type":"text","text":"OK"}]},
			{"id":"msg_u","time":{"created":50},"type":"user","text":"hello"},
			{"id":"msg_idle","time":{"created":300},"type":"idle","outcome":"succeeded"}]}`))
	})
	mux.HandleFunc("POST /api/session/ses_new/prompt", func(w http.ResponseWriter, r *http.Request) {
		f.record(r)
		raw, _ := io.ReadAll(r.Body)
		f.mu.Lock()
		f.bodies["prompt"] = string(raw)
		f.mu.Unlock()
		w.WriteHeader(http.StatusNoContent)
	})
	mux.HandleFunc("POST /api/session/ses_new/model", func(w http.ResponseWriter, r *http.Request) {
		f.record(r)
		raw, _ := io.ReadAll(r.Body)
		f.mu.Lock()
		f.bodies["model"] = string(raw)
		f.mu.Unlock()
		w.WriteHeader(http.StatusNoContent)
	})
	mux.HandleFunc("POST /api/session/ses_new/agent", func(w http.ResponseWriter, r *http.Request) {
		f.record(r)
		w.WriteHeader(http.StatusNoContent)
	})
	mux.HandleFunc("POST /api/session/ses_new/interrupt", func(w http.ResponseWriter, r *http.Request) {
		f.record(r)
		w.WriteHeader(http.StatusNoContent)
	})
	mux.HandleFunc("POST /api/session/ses_new/permission/per_1/reply", func(w http.ResponseWriter, r *http.Request) {
		f.record(r)
		w.WriteHeader(http.StatusNoContent)
	})
	mux.HandleFunc("GET /api/event", func(w http.ResponseWriter, r *http.Request) {
		f.record(r)
		w.Header().Set("Content-Type", "text/event-stream")
		flusher := w.(http.Flusher)
		_, _ = fmt.Fprint(w, "data: {\"id\":\"evt_0\",\"type\":\"server.connected\",\"data\":{}}\n\n")
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

func (f *fakeOC) body(key string) string {
	f.mu.Lock()
	defer f.mu.Unlock()
	return f.bodies[key]
}

// pushEvent queues a raw v2 envelope (as the SSE data line) to stream.
func (f *fakeOC) pushEvent(raw string) {
	f.t.Helper()
	select {
	case f.stream <- raw:
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

func envelopeTypes(out []protocol.Envelope) []string {
	types := make([]string, 0, len(out))
	for _, e := range out {
		types = append(types, e.Type)
	}
	return types
}

func payloadAs(t *testing.T, e protocol.Envelope, dst any) {
	t.Helper()
	if err := json.Unmarshal(e.Payload, dst); err != nil {
		t.Fatalf("unmarshal %s payload: %v", e.Type, err)
	}
}

func v2Event(typ string, data map[string]any) string {
	raw, _ := json.Marshal(map[string]any{"id": "evt_x", "type": typ, "data": data})
	return string(raw)
}

func TestAdapter_Attach_LifecycleAndTurn(t *testing.T) {
	f := startFake(t)
	a := NewAdapter(AdapterConfig{ProjectID: "p1", Mode: ModeAttach, URL: f.url})

	ctx := context.Background()
	if err := a.Start(ctx); err != nil {
		t.Fatal(err)
	}

	// First event must be adapter.status running (from server.connected).
	evts := drain(t, a.Events(), func(e protocol.Envelope) bool {
		return e.Type == protocol.EventAdapterStatus
	})
	var st protocol.AdapterStatus
	payloadAs(t, evts[len(evts)-1], &st)
	if st.State != protocol.AdapterRunning || st.ProjectID != "p1" {
		t.Errorf("status = %+v", st)
	}

	// Stream a v2 turn: session created in-scope, then text started→delta→ended.
	f.pushEvent(v2Event("session.created", map[string]any{
		"sessionID": "ses_new", "projectID": "prj",
		"location": map[string]any{"directory": "/private/tmp/proj"}}))
	f.pushEvent(v2Event("session.text.started", map[string]any{
		"sessionID": "ses_new", "assistantMessageID": "msg_1"}))
	f.pushEvent(v2Event("session.text.delta", map[string]any{
		"sessionID": "ses_new", "assistantMessageID": "msg_1", "delta": "OK"}))
	f.pushEvent(v2Event("session.text.ended", map[string]any{
		"sessionID": "ses_new", "assistantMessageID": "msg_1", "text": "OK"}))

	// Wait for the authoritative ended part; the stub + delta precede it.
	var texts []protocol.Part
	deadline := time.After(5 * time.Second)
 collecting:
	for {
		select {
		case e := <-a.Events():
			if e.Type == protocol.EventPartUpdated {
				var pu protocol.PartUpdated
				payloadAs(t, e, &pu)
				if pu.Part.Type == protocol.PartText {
					texts = append(texts, pu.Part)
					if pu.Part.Text == "OK" {
						break collecting
					}
				}
			}
		case <-deadline:
			t.Fatalf("never got the full text part; got %v", texts)
		}
	}
	// Ended replaces: first paint is empty, last is authoritative.
	if texts[0].Text != "" || texts[len(texts)-1].Text != "OK" {
		t.Errorf("text parts = %+v", texts)
	}
	// Part ids are stable across started/delta/ended.
	for _, p := range texts {
		if p.ID != "msg_1:text" {
			t.Errorf("part id = %s", p.ID)
		}
	}

	// Commands proxy through the v2 client: model pinned, then text prompt.
	if err := a.Prompt(ctx, "ses_new", protocol.PromptRequest{
		Text: "hi", Agent: "build", Provider: "zai-coding-plan", Model: "glm-5.3", Variant: "high",
	}); err != nil {
		t.Fatal(err)
	}
	if !f.hasRequest("POST", "/api/session/ses_new/model") {
		t.Error("model pin not called")
	}
	var modelBody struct {
		Model map[string]string `json:"model"`
	}
	if err := json.Unmarshal([]byte(f.body("model")), &modelBody); err != nil {
		t.Fatal(err)
	}
	if modelBody.Model["id"] != "glm-5.3" || modelBody.Model["providerID"] != "zai-coding-plan" || modelBody.Model["variant"] != "high" {
		t.Errorf("model body = %+v", modelBody.Model)
	}
	if !f.hasRequest("POST", "/api/session/ses_new/agent") {
		t.Error("agent pin not called")
	}
	if !f.hasRequest("POST", "/api/session/ses_new/prompt") {
		t.Error("prompt not called")
	}
	var promptBody struct {
		Text string `json:"text"`
	}
	if err := json.Unmarshal([]byte(f.body("prompt")), &promptBody); err != nil {
		t.Fatal(err)
	}
	if promptBody.Text != "hi" {
		t.Errorf("prompt body = %+v", promptBody)
	}
	if err := a.Abort(ctx, "ses_new"); err != nil {
		t.Fatal(err)
	}
	if !f.hasRequest("POST", "/api/session/ses_new/interrupt") {
		t.Error("interrupt not called")
	}
	if err := a.ReplyPermission(ctx, "ses_new", "per_1", protocol.PermissionOnce); err != nil {
		t.Fatal(err)
	}
	if !f.hasRequest("POST", "/api/session/ses_new/permission/per_1/reply") {
		t.Error("permission reply not called")
	}

	ss, err := a.Sessions(ctx)
	if err != nil || len(ss) != 2 || ss[0].Title != "one" {
		t.Errorf("sessions = %+v err=%v", ss, err)
	}

	// Hydration: user/assistant inline content, idle markers dropped, finish
	// part synthesized, tool state mapped.
	msgs, err := a.Messages(ctx, "ses_h", 0)
	if err != nil {
		t.Fatal(err)
	}
	if len(msgs) != 2 {
		t.Fatalf("messages = %d, want 2 (idle marker dropped)", len(msgs))
	}
	// Oldest first: user, then assistant.
	if msgs[0].Info.Role != protocol.RoleUser || msgs[0].Parts[0].Text != "hello" {
		t.Errorf("user message = %+v", msgs[0])
	}
	aMsg := msgs[1]
	if aMsg.Info.Role != protocol.RoleAssistant || aMsg.Info.Finish != "stop" {
		t.Errorf("assistant info = %+v", aMsg.Info)
	}
	var ptypes []string
	for _, p := range aMsg.Parts {
		ptypes = append(ptypes, p.Type)
	}
	want := []string{protocol.PartReasoning, protocol.PartTool, protocol.PartText, protocol.PartStepFinish}
	if strings.Join(ptypes, ",") != strings.Join(want, ",") {
		t.Errorf("hydrated parts = %v, want %v", ptypes, want)
	}
	if aMsg.Parts[1].State == nil || aMsg.Parts[1].State.Status != protocol.ToolCompleted {
		t.Errorf("tool state = %+v", aMsg.Parts[1].State)
	}
	if aMsg.Parts[3].Tokens == nil || aMsg.Parts[3].Tokens.Output != 2 {
		t.Errorf("finish tokens = %+v", aMsg.Parts[3].Tokens)
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

// The OpenCode session store is global: the session list and the event
// stream carry sessions from every directory. The adapter must scope them
// to the server's root (GET /api/location → directory).
func TestAdapter_SessionScoping(t *testing.T) {
	root := "/private/tmp/proj"

	mux := http.NewServeMux()
	mux.HandleFunc("GET /api/info", func(w http.ResponseWriter, r *http.Request) {
		_ = json.NewEncoder(w).Encode(V2ServerInfo{Version: "test"})
	})
	mux.HandleFunc("GET /api/location", func(w http.ResponseWriter, r *http.Request) {
		_, _ = w.Write([]byte(`{"directory":"/private/tmp/proj","project":{"id":"prj"}}`))
	})
	mux.HandleFunc("GET /api/session", func(w http.ResponseWriter, r *http.Request) {
		_, _ = w.Write([]byte(`{"data":[
			{"id":"ses_in","projectID":"prj","title":"in project","time":{"created":1,"updated":1},"location":{"directory":"/private/tmp/proj"}},
			{"id":"ses_tmp","projectID":"prj","title":"via symlink","time":{"created":1,"updated":1},"location":{"directory":"/tmp/proj"}},
			{"id":"ses_out","projectID":"prj","title":"other project","time":{"created":1,"updated":1},"location":{"directory":"/Users/x/other"}}]}`))
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

	// A session.created from a foreign directory must not reach Events().
	foreign := v2Event("session.created", map[string]any{
		"sessionID": "ses_out", "location": map[string]any{"directory": "/Users/x/other"}})
	env, err := parseV2Event([]byte(foreign))
	if err != nil {
		t.Fatal(err)
	}
	if !a.foreignSessionEvent(env) {
		t.Error("foreign session.created not detected")
	}
	ours := v2Event("session.created", map[string]any{
		"sessionID": "ses_in", "location": map[string]any{"directory": root}})
	env, _ = parseV2Event([]byte(ours))
	if a.foreignSessionEvent(env) {
		t.Error("in-scope session.created wrongly flagged foreign")
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

// Integration: full managed lifecycle against a real opencode v2 binary.
// Skipped when no v2 binary is available or -short is passed.
func TestAdapter_Managed_Integration_OpencodeV2(t *testing.T) {
	if testing.Short() {
		t.Skip("short mode")
	}
	bin := os.Getenv("CIRCULOGO_OPENCODE_BIN")
	if bin == "" {
		candidate := os.ExpandEnv("$HOME/.local/opencode-v2/node_modules/@opencode/cli-darwin-arm64/bin/opencode")
		if _, err := os.Stat(candidate); err != nil {
			t.Skip("no opencode v2 binary available")
		}
		bin = candidate
	}
	a := NewAdapter(AdapterConfig{ProjectID: "p", Mode: ModeManaged, Dir: t.TempDir(), Binary: bin})
	if err := a.Start(context.Background()); err != nil {
		t.Fatal(err)
	}
	// First event: adapter running (after boot password + /api/info + /event).
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
