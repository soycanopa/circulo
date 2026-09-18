package opencode

import (
	"context"
	"encoding/json"
	"io"
	"net/http"
	"net/http/httptest"
	"testing"
)

// Shapes below come from a live 2.0.8 server (docs/opencode-v2-migration.md).

func TestClientV2_SessionsUnwrapsDataEnvelope(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/api/session" {
			t.Errorf("path = %s", r.URL.Path)
		}
		_, _ = w.Write([]byte(`{"location":{"directory":"/tmp/p"},"data":[
			{"id":"ses_1","projectID":"prj","title":"t","time":{"created":100,"updated":200},
			 "location":{"directory":"/tmp/p"},"cost":0.5,
			 "tokens":{"input":1,"output":2,"reasoning":3,"cache":{"read":4,"write":5}}},
			{"id":"ses_2","projectID":"prj","title":"other","time":{"created":1,"updated":2},
			 "location":{"directory":"/other"}}]}`))
	}))
	defer srv.Close()

	c := NewClient(srv.URL)
	sessions, err := c.SessionsV2(context.Background())
	if err != nil {
		t.Fatal(err)
	}
	if len(sessions) != 2 {
		t.Fatalf("sessions = %d", len(sessions))
	}
	s := sessions[0]
	if s.ID != "ses_1" || s.Title != "t" || s.Time.Updated != 200 || s.Location.Directory != "/tmp/p" {
		t.Errorf("session = %+v", s)
	}
	if s.Tokens == nil || s.Tokens.Cache.Read != 4 {
		t.Errorf("tokens = %+v", s.Tokens)
	}
}

func TestClientV2_ModelsCarryVariantsAndEnabled(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		_, _ = w.Write([]byte(`{"data":[
			{"id":"glm-5.3","modelID":"glm-5.3","providerID":"zai-coding-plan","name":"GLM-5.3",
			 "capabilities":{"tools":true,"input":["text"],"output":["text"]},
			 "variants":[{"id":"low"},{"id":"high"},{"id":"max"}],"enabled":true},
			{"id":"glm-4.7","modelID":"glm-4.7","providerID":"zai-coding-plan","name":"GLM-4.7",
			 "capabilities":{"tools":true,"input":["text"],"output":["text"]},"enabled":true}]}`))
	}))
	defer srv.Close()

	c := NewClient(srv.URL)
	models, err := c.ModelsV2(context.Background())
	if err != nil {
		t.Fatal(err)
	}
	if len(models) != 2 {
		t.Fatalf("models = %d", len(models))
	}
	if len(models[0].Variants) != 3 || models[0].Variants[0].ID != "low" {
		t.Errorf("variants = %+v", models[0].Variants)
	}
	if !models[0].Enabled {
		t.Error("enabled must survive parsing")
	}
	// Effort selection signal in v2 is the variants list (no reasoning flag).
	if len(models[1].Variants) != 0 {
		t.Errorf("glm-4.7 must have no variants, got %+v", models[1].Variants)
	}
}

func TestClientV2_MessagesCarryInlineContent(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Query().Get("limit") != "50" {
			t.Errorf("limit = %s", r.URL.Query().Get("limit"))
		}
		_, _ = w.Write([]byte(`{"data":[
			{"id":"msg_a","time":{"created":1,"completed":2},"type":"assistant","agent":"build",
			 "model":{"id":"m","providerID":"p"},"finish":"stop","cost":0.01,
			 "tokens":{"input":1,"output":2,"reasoning":0,"cache":{"read":0,"write":0}},
			 "content":[
			   {"type":"reasoning","text":"thinking","time":{"created":1,"completed":2}},
			   {"type":"tool","id":"call_1","name":"shell",
			    "state":{"status":"completed","input":{"command":"ls"},"output":"x"},
			    "time":{"created":1,"completed":2}},
			   {"type":"text","text":"DONE"}]},
			{"id":"msg_u","time":{"created":1},"type":"user","text":"run ls"}]}`))
	}))
	defer srv.Close()

	c := NewClient(srv.URL)
	msgs, err := c.MessagesV2(context.Background(), "ses_1", 50)
	if err != nil {
		t.Fatal(err)
	}
	if len(msgs) != 2 {
		t.Fatalf("messages = %d", len(msgs))
	}
	a := msgs[0]
	if len(a.Content) != 3 {
		t.Fatalf("content = %+v", a.Content)
	}
	if a.Content[1].Type != "tool" || a.Content[1].State.Status != "completed" || a.Content[1].Name != "shell" {
		t.Errorf("tool content = %+v", a.Content[1])
	}
	if a.Tokens == nil || a.Tokens.Output != 2 {
		t.Errorf("tokens = %+v", a.Tokens)
	}
	if msgs[1].Type != "user" || msgs[1].Text != "run ls" {
		t.Errorf("user message = %+v", msgs[1])
	}
}

// Regression: the real 2.0.8 REST history carries tool errors as
// Session.StructuredError objects and results as content[] — a rigid string
// decode failed the WHOLE messages call, killing history hydration
// (docs/opencode-v2-migration.md; owner session "Saludo", 2026-09-18).
func TestClientV2_MessagesTolerateStructuredToolErrors(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		_, _ = w.Write([]byte(`{"data":[
			{"id":"msg_a","time":{"created":1},"type":"assistant",
			 "content":[
			   {"type":"tool","id":"call_1","name":"shell",
			    "state":{"status":"error","input":{"command":"boom"},
			     "error":{"type":"provider.no-route","message":"Model unavailable: x/y","status":0}}},
			   {"type":"tool","id":"call_2","name":"read",
			    "state":{"status":"completed","input":{"file_path":"a.ts"},
			     "content":[{"type":"text","text":"line one"},{"type":"text","text":"line two"}]}}]}]}`))
	}))
	defer srv.Close()

	c := NewClient(srv.URL)
	msgs, err := c.MessagesV2(context.Background(), "ses_1", 0)
	if err != nil {
		t.Fatalf("decode failed: %v", err)
	}
	toolErr := msgs[0].Content[0].stateError()
	if toolErr != "Model unavailable: x/y" {
		t.Errorf("stateError = %q", toolErr)
	}
	out := msgs[0].Content[1].stateOutput()
	if out != "line one\nline two" {
		t.Errorf("stateOutput = %q", out)
	}
}

func TestClientV2_PromptBodyIsTextOnly(t *testing.T) {
	var gotBody map[string]any
	var gotPath, gotMethod string
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		gotPath, gotMethod = r.URL.Path, r.Method
		raw, _ := io.ReadAll(r.Body)
		_ = json.Unmarshal(raw, &gotBody)
		w.WriteHeader(http.StatusNoContent)
	}))
	defer srv.Close()

	c := NewClient(srv.URL)
	if err := c.PromptV2(context.Background(), "ses_1", "hi"); err != nil {
		t.Fatal(err)
	}
	if gotPath != "/api/session/ses_1/prompt" || gotMethod != http.MethodPost {
		t.Errorf("request = %s %s", gotMethod, gotPath)
	}
	// additionalProperties:false — only text is allowed.
	if len(gotBody) != 1 || gotBody["text"] != "hi" {
		t.Errorf("body = %+v", gotBody)
	}
}

func TestClientV2_SetModelShape(t *testing.T) {
	var gotBody map[string]any
	var gotPath string
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		gotPath = r.URL.Path
		raw, _ := io.ReadAll(r.Body)
		_ = json.Unmarshal(raw, &gotBody)
		w.WriteHeader(http.StatusNoContent)
	}))
	defer srv.Close()

	c := NewClient(srv.URL)
	if err := c.SetModelV2(context.Background(), "ses_1", "zai-coding-plan", "glm-5.3", "high"); err != nil {
		t.Fatal(err)
	}
	if gotPath != "/api/session/ses_1/model" {
		t.Errorf("path = %s", gotPath)
	}
	model, _ := gotBody["model"].(map[string]any)
	if model["id"] != "glm-5.3" || model["providerID"] != "zai-coding-plan" || model["variant"] != "high" {
		t.Errorf("model = %+v", model)
	}
}

func TestClientV2_ReplyPermissionUsesDecisionKey(t *testing.T) {
	var gotBody map[string]any
	var gotPath string
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		gotPath = r.URL.Path
		raw, _ := io.ReadAll(r.Body)
		_ = json.Unmarshal(raw, &gotBody)
		w.WriteHeader(http.StatusNoContent)
	}))
	defer srv.Close()

	c := NewClient(srv.URL)
	if err := c.ReplyPermissionV2(context.Background(), "ses_1", "per_1", "always"); err != nil {
		t.Fatal(err)
	}
	if gotPath != "/api/session/ses_1/permission/per_1/reply" {
		t.Errorf("path = %s", gotPath)
	}
	if gotBody["decision"] != "always" {
		t.Errorf("body = %+v", gotBody)
	}
}

// Session instruction entries (experimental surface): PUT path + {value} body.
func TestClientV2_PutInstructionEntry(t *testing.T) {
	var gotBody map[string]any
	var gotPath, gotMethod string
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		gotPath, gotMethod = r.URL.Path, r.Method
		raw, _ := io.ReadAll(r.Body)
		_ = json.Unmarshal(raw, &gotBody)
		w.WriteHeader(http.StatusNoContent)
	}))
	defer srv.Close()

	c := NewClient(srv.URL)
	if err := c.PutInstructionV2(context.Background(), "ses_1", "circulogo-format", "use markdown"); err != nil {
		t.Fatal(err)
	}
	if gotMethod != http.MethodPut ||
		gotPath != "/api/experimental/session/ses_1/instructions/entries/circulogo-format" {
		t.Errorf("request = %s %s", gotMethod, gotPath)
	}
	if gotBody["value"] != "use markdown" {
		t.Errorf("body = %+v", gotBody)
	}
}

func TestClientV2_ServerInfoAndLocation(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		switch r.URL.Path {
		case "/api/info":
			_, _ = w.Write([]byte(`{"version":"2.0.8","pid":11,"urls":["http://127.0.0.1:1"]}`))
		case "/api/location":
			_, _ = w.Write([]byte(`{"directory":"/tmp/p","project":{"id":"prj","directory":"/tmp/p","canonical":"/tmp/p"}}`))
		default:
			t.Errorf("unexpected path %s", r.URL.Path)
		}
	}))
	defer srv.Close()

	c := NewClient(srv.URL)
	info, err := c.ServerInfoV2(context.Background())
	if err != nil || info.Version != "2.0.8" {
		t.Errorf("info = %+v, err %v", info, err)
	}
	loc, err := c.LocationV2(context.Background())
	if err != nil || loc.Directory != "/tmp/p" || loc.Project.ID != "prj" {
		t.Errorf("location = %+v, err %v", loc, err)
	}
}

func TestClientV2_AgentsUnwrapAndHidden(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		_, _ = w.Write([]byte(`{"data":[
			{"id":"build","name":"Build","description":"default","mode":"primary","hidden":false},
			{"id":"explore","name":"Explore","mode":"subagent","hidden":true}]}`))
	}))
	defer srv.Close()

	c := NewClient(srv.URL)
	agents, err := c.AgentsV2(context.Background())
	if err != nil {
		t.Fatal(err)
	}
	if len(agents) != 2 || agents[0].ID != "build" || agents[0].Mode != "primary" {
		t.Errorf("agents = %+v", agents)
	}
	if !agents[1].Hidden {
		t.Errorf("hidden flag lost")
	}
}

func TestClientV2_ParseEventEnvelope(t *testing.T) {
	line := []byte(`{"id":"evt_1","created":1789750349764,"type":"session.created","data":{"sessionID":"ses_9"}}`)
	env, err := parseV2Event(line)
	if err != nil {
		t.Fatal(err)
	}
	if env.Type != "session.created" || env.ID != "evt_1" {
		t.Errorf("envelope = %+v", env)
	}
	var payload struct {
		SessionID string `json:"sessionID"`
	}
	if err := json.Unmarshal(env.Data, &payload); err != nil {
		t.Fatal(err)
	}
	if payload.SessionID != "ses_9" {
		t.Errorf("payload = %+v", payload)
	}
}

func TestClientV2_BasicAuthOnV2Routes(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		user, pass, ok := r.BasicAuth()
		if !ok || user != "opencode" || pass != "secret" {
			w.WriteHeader(http.StatusUnauthorized)
			return
		}
		_, _ = w.Write([]byte(`{"data":[]}`))
	}))
	defer srv.Close()

	c := NewClient(srv.URL, WithBasicAuth("opencode", "secret"))
	if _, err := c.SessionsV2(context.Background()); err != nil {
		t.Fatalf("authenticated call failed: %v", err)
	}
}
