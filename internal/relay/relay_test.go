package relay_test

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"net/http"
	"net/http/httptest"
	"strings"
	"sync"
	"testing"
	"time"

	"circulogo/internal/agent"
	"circulogo/internal/agent/protocol"
	"circulogo/internal/orchestrator"
	"circulogo/internal/relay"
	"circulogo/internal/store"
)

// fakeAdapter implements agent.Adapter for relay tests. Only the methods the
// relay exercises do something.
type fakeAdapter struct {
	mu       sync.Mutex
	prompt   protocol.PromptRequest
	permResp string
	renamed  [2]string // sessionID, new title
	startErr error
	events   chan protocol.Envelope
}

func newFakeAdapter() *fakeAdapter { return &fakeAdapter{events: make(chan protocol.Envelope, 64)} }

func (f *fakeAdapter) Start(_ context.Context) error {
	if f.startErr != nil {
		return f.startErr
	}
	env, _ := protocol.NewEnvelope(protocol.EventAdapterStatus, protocol.AdapterStatus{
		ProjectID: "x", State: protocol.AdapterRunning,
	})
	f.events <- env
	return nil
}
func (f *fakeAdapter) Stop(_ context.Context) error { return nil }
func (f *fakeAdapter) Events() <-chan protocol.Envelope {
	return f.events
}
func (f *fakeAdapter) Sessions(_ context.Context) ([]protocol.Session, error) {
	return []protocol.Session{{ID: "ses_1", Title: "first", TimeCreated: 100, TimeUpdated: 200}}, nil
}
func (f *fakeAdapter) CreateSession(_ context.Context, title string) (protocol.Session, error) {
	return protocol.Session{ID: "ses_new", Title: title}, nil
}
func (f *fakeAdapter) RenameSession(_ context.Context, sessionID, title string) error {
	f.mu.Lock()
	f.renamed = [2]string{sessionID, title}
	f.mu.Unlock()
	return nil
}
func (f *fakeAdapter) DeleteSession(_ context.Context, _ string) error { return nil }
func (f *fakeAdapter) Messages(_ context.Context, _ string, _ int) ([]agent.HydratedMessage, error) {
	return []agent.HydratedMessage{{
		Info:  protocol.MessageInfo{ID: "msg_1", Role: protocol.RoleUser},
		Parts: []protocol.Part{{ID: "prt_1", Type: protocol.PartText, Text: "hello"}},
	}}, nil
}
func (f *fakeAdapter) Prompt(_ context.Context, _ string, req protocol.PromptRequest) error {
	f.mu.Lock()
	f.prompt = req
	f.mu.Unlock()
	return nil
}
func (f *fakeAdapter) Abort(_ context.Context, _ string) error { return nil }
func (f *fakeAdapter) ReplyPermission(_ context.Context, _, _, response string) error {
	f.mu.Lock()
	f.permResp = response
	f.mu.Unlock()
	return nil
}
func (f *fakeAdapter) ReplyForm(_ context.Context, _, _ string, _ map[string]any) error {
	return nil
}
func (f *fakeAdapter) Vcs(_ context.Context) protocol.ProjectVcs {
	return protocol.ProjectVcs{IsRepo: true, Provider: "git", Branch: "main"}
}
func (f *fakeAdapter) Branches(_ context.Context) []string            { return []string{"main"} }
func (f *fakeAdapter) SetBranch(_ context.Context, _, _ string) error { return nil }

func (f *fakeAdapter) Meta(_ context.Context) (protocol.Meta, error) {
	return protocol.Meta{
		Agents: []protocol.AgentInfo{{Name: "build", Mode: "primary"}},
		Models: []protocol.ModelInfo{{ID: "glm-5.3", Provider: "zai-coding-plan"}},
	}, nil
}

func (f *fakeAdapter) AccessSupported() bool                       { return false }
func (f *fakeAdapter) SetAccess(_ context.Context, _ string) error { return nil }
func (f *fakeAdapter) RunCommand(_ context.Context, _, _, _ string) error {
	return nil
}

type factory struct {
	mu   sync.Mutex
	made map[string]*fakeAdapter
}

func (ff *factory) build(cfg store.Project) (agent.Adapter, error) {
	ff.mu.Lock()
	defer ff.mu.Unlock()
	a := newFakeAdapter()
	if strings.HasSuffix(cfg.URL, ":1") {
		a.startErr = errors.New("down")
	}
	ff.made[cfg.Path] = a
	return a, nil
}

func newServer(t *testing.T) (*httptest.Server, *factory) {
	t.Helper()
	ff := &factory{made: map[string]*fakeAdapter{}}
	orch := orchestrator.New(store.New(t.TempDir()+"/settings.json"), ff.build)
	mux := http.NewServeMux()
	// Same mounting as main.go: relay serves prefix-less paths.
	mux.Handle("/agent/", http.StripPrefix("/agent", relay.New(orch)))
	srv := httptest.NewServer(mux)
	t.Cleanup(srv.Close)
	t.Cleanup(orch.Shutdown)
	return srv, ff
}

func addProject(t *testing.T, srv *httptest.Server, path string) map[string]any {
	t.Helper()
	body := fmt.Sprintf(`{"path":%q,"mode":"attach","url":"http://127.0.0.1:4096"}`, path)
	resp, err := http.Post(srv.URL+"/agent/projects", "application/json", strings.NewReader(body))
	if err != nil {
		t.Fatal(err)
	}
	defer resp.Body.Close()
	if resp.StatusCode != http.StatusOK {
		t.Fatalf("add project status = %d", resp.StatusCode)
	}
	var out map[string]any
	_ = json.NewDecoder(resp.Body).Decode(&out)
	return out
}

func TestProjectsRoundTrip(t *testing.T) {
	srv, _ := newServer(t)

	pv := addProject(t, srv, t.TempDir())
	if pv["status"] != protocol.AdapterStarting {
		t.Errorf("new project = %+v, want starting", pv)
	}

	resp, err := http.Get(srv.URL + "/agent/projects")
	if err != nil {
		t.Fatal(err)
	}
	defer resp.Body.Close()
	var list []map[string]any
	_ = json.NewDecoder(resp.Body).Decode(&list)
	if len(list) != 1 {
		t.Fatalf("projects = %+v", list)
	}
}

func TestSSEStreamsNeutralEvents(t *testing.T) {
	srv, _ := newServer(t)
	pv := addProject(t, srv, t.TempDir())
	id := pv["id"].(string)

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	req, _ := http.NewRequestWithContext(ctx, http.MethodGet, srv.URL+"/agent/sse", nil)
	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		t.Fatal(err)
	}
	defer resp.Body.Close()

	// The replay ring must contain this project's adapter.status (running)
	// even though we connected after it fired.
	deadline := time.After(4 * time.Second)
	sc := make(chan string, 64)
	go func() {
		buf := make([]byte, 4096)
		for {
			n, err := resp.Body.Read(buf)
			if n > 0 {
				sc <- string(buf[:n])
			}
			if err != nil {
				close(sc)
				return
			}
		}
	}()
	var acc strings.Builder
	for {
		select {
		case chunk, ok := <-sc:
			if !ok {
				t.Fatal("sse stream ended early")
			}
			acc.WriteString(chunk)
			if strings.Contains(acc.String(), `"adapter.status"`) &&
				strings.Contains(acc.String(), `"running"`) &&
				strings.Contains(acc.String(), id) {
				return // success: replayed status reached the client
			}
		case <-deadline:
			t.Fatalf("no adapter.status on sse; got: %s", acc.String())
		}
	}
}

func TestPromptRejectsEmptyText(t *testing.T) {
	srv, _ := newServer(t)
	pv := addProject(t, srv, t.TempDir())
	id := pv["id"].(string)

	// Wait until the adapter is wired so AdapterOf succeeds.
	waitRunning(t, srv, id)

	resp, err := http.Post(srv.URL+"/agent/projects/"+id+"/sessions/ses_1/prompt",
		"application/json", strings.NewReader(`{"text":""}`))
	if err != nil {
		t.Fatal(err)
	}
	defer resp.Body.Close()
	if resp.StatusCode != http.StatusBadRequest {
		t.Fatalf("status = %d, want 400", resp.StatusCode)
	}
}

func waitRunning(t *testing.T, srv *httptest.Server, id string) {
	t.Helper()
	deadline := time.Now().Add(3 * time.Second)
	for time.Now().Before(deadline) {
		resp, err := http.Get(srv.URL + "/agent/projects/" + id)
		if err == nil {
			var pv map[string]any
			_ = json.NewDecoder(resp.Body).Decode(&pv)
			resp.Body.Close()
			if pv["status"] == protocol.AdapterRunning {
				return
			}
		}
		time.Sleep(20 * time.Millisecond)
	}
	t.Fatal("project never reached running")
}

func TestPromptProxiesToAdapter(t *testing.T) {
	srv, ff := newServer(t)
	dir := t.TempDir()
	pv := addProject(t, srv, dir)
	id := pv["id"].(string)
	waitRunning(t, srv, id)

	resp, err := http.Post(srv.URL+"/agent/projects/"+id+"/sessions/ses_1/prompt",
		"application/json", strings.NewReader(`{"text":"do it","agent":"build","provider":"p","model":"m","variant":"high"}`))
	if err != nil {
		t.Fatal(err)
	}
	resp.Body.Close()
	if resp.StatusCode != http.StatusOK {
		t.Fatalf("prompt status = %d", resp.StatusCode)
	}
	a := ff.made[dir]
	a.mu.Lock()
	defer a.mu.Unlock()
	if a.prompt.Text != "do it" || a.prompt.Agent != "build" || a.prompt.Variant != "high" {
		t.Errorf("adapter got %+v", a.prompt)
	}
}

func TestPermissionReplyValidated(t *testing.T) {
	srv, ff := newServer(t)
	dir := t.TempDir()
	pv := addProject(t, srv, dir)
	id := pv["id"].(string)
	waitRunning(t, srv, id)

	url := srv.URL + "/agent/projects/" + id + "/sessions/ses_1/permissions/per_9"
	resp, err := http.Post(url, "application/json", strings.NewReader(`{"response":"sometimes"}`))
	if err != nil {
		t.Fatal(err)
	}
	resp.Body.Close()
	if resp.StatusCode != http.StatusBadRequest {
		t.Fatalf("invalid response status = %d, want 400", resp.StatusCode)
	}

	resp, err = http.Post(url, "application/json", strings.NewReader(`{"response":"always"}`))
	if err != nil {
		t.Fatal(err)
	}
	resp.Body.Close()
	if resp.StatusCode != http.StatusOK {
		t.Fatalf("valid response status = %d", resp.StatusCode)
	}
	a := ff.made[dir]
	a.mu.Lock()
	defer a.mu.Unlock()
	if a.permResp != protocol.PermissionAlways {
		t.Errorf("adapter got %q", a.permResp)
	}
}

func TestMetaAndSessionsAndHydration(t *testing.T) {
	srv, _ := newServer(t)
	pv := addProject(t, srv, t.TempDir())
	id := pv["id"].(string)
	waitRunning(t, srv, id)

	for _, path := range []string{
		"/agent/projects/" + id + "/meta",
		"/agent/projects/" + id + "/sessions",
		"/agent/projects/" + id + "/sessions/ses_1/messages",
	} {
		resp, err := http.Get(srv.URL + path)
		if err != nil {
			t.Fatal(err)
		}
		if resp.StatusCode != http.StatusOK {
			t.Errorf("GET %s = %d", path, resp.StatusCode)
		}
		resp.Body.Close()
	}
}

func TestRenameSessionProxiesToAdapter(t *testing.T) {
	srv, ff := newServer(t)
	dir := t.TempDir()
	pv := addProject(t, srv, dir)
	id := pv["id"].(string)
	waitRunning(t, srv, id)

	url := srv.URL + "/agent/projects/" + id + "/sessions/ses_1"

	// Empty/blank titles are rejected before reaching the adapter.
	for _, body := range []string{`{"title":""}`, `{"title":"   "}`, `not json`} {
		req, _ := http.NewRequest(http.MethodPatch, url, strings.NewReader(body))
		resp, err := http.DefaultClient.Do(req)
		if err != nil {
			t.Fatal(err)
		}
		resp.Body.Close()
		if resp.StatusCode != http.StatusBadRequest {
			t.Fatalf("PATCH %q status = %d, want 400", body, resp.StatusCode)
		}
	}

	req, _ := http.NewRequest(http.MethodPatch, url, strings.NewReader(`{"title":"  renamed  "}`))
	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		t.Fatal(err)
	}
	resp.Body.Close()
	if resp.StatusCode != http.StatusOK {
		t.Fatalf("rename status = %d", resp.StatusCode)
	}
	a := ff.made[dir]
	a.mu.Lock()
	defer a.mu.Unlock()
	if a.renamed != [2]string{"ses_1", "renamed"} {
		t.Errorf("adapter rename = %v", a.renamed)
	}
}

func TestUnknownProjectIs404ish(t *testing.T) {
	srv, _ := newServer(t)
	resp, err := http.Get(srv.URL + "/agent/projects/nope/sessions")
	if err != nil {
		t.Fatal(err)
	}
	defer resp.Body.Close()
	if resp.StatusCode == http.StatusOK {
		t.Fatal("unknown project must not return 200")
	}
}
