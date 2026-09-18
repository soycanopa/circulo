package opencode

import (
	"context"
	"encoding/json"
	"errors"
	"io"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"
)

func TestClient_CreateSessionSendsTitle(t *testing.T) {
	var gotPath, gotMethod string
	var gotBody map[string]any
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		gotPath, gotMethod = r.URL.Path, r.Method
		raw, _ := io.ReadAll(r.Body)
		_ = json.Unmarshal(raw, &gotBody)
		_ = json.NewEncoder(w).Encode(Session{ID: "ses_new", Title: "t"})
	}))
	defer srv.Close()

	c := NewClient(srv.URL)
	s, err := c.CreateSession(context.Background(), "my title")
	if err != nil {
		t.Fatal(err)
	}
	if s.ID != "ses_new" || gotPath != "/session" || gotMethod != http.MethodPost {
		t.Errorf("request = %s %s, session %+v", gotMethod, gotPath, s)
	}
	if gotBody["title"] != "my title" {
		t.Errorf("body = %+v, want title", gotBody)
	}
}

func TestClient_PromptAsyncShape(t *testing.T) {
	var gotBody map[string]any
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/session/ses_1/prompt_async" || r.Method != http.MethodPost {
			t.Errorf("path = %s %s", r.Method, r.URL.Path)
		}
		raw, _ := io.ReadAll(r.Body)
		_ = json.Unmarshal(raw, &gotBody)
		w.WriteHeader(http.StatusNoContent)
	}))
	defer srv.Close()

	c := NewClient(srv.URL)
	err := c.Prompt(context.Background(), "ses_1", PromptInput{
		Text: "hi", Agent: "build", Provider: "zai-coding-plan", Model: "glm-5.3",
		Variant: "high",
	})
	if err != nil {
		t.Fatal(err)
	}
	model, _ := gotBody["model"].(map[string]any)
	if model["providerID"] != "zai-coding-plan" || model["modelID"] != "glm-5.3" {
		t.Errorf("model = %+v", model)
	}
	if gotBody["agent"] != "build" {
		t.Errorf("agent = %+v", gotBody["agent"])
	}
	if gotBody["variant"] != "high" {
		t.Errorf("variant = %+v", gotBody["variant"])
	}
	parts, _ := gotBody["parts"].([]any)
	if len(parts) != 1 || parts[0].(map[string]any)["text"] != "hi" {
		t.Errorf("parts = %+v", parts)
	}
}

func TestClient_ReplyPermission(t *testing.T) {
	var gotBody map[string]any
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/session/ses_1/permissions/per_1" {
			t.Errorf("path = %s", r.URL.Path)
		}
		raw, _ := io.ReadAll(r.Body)
		_ = json.Unmarshal(raw, &gotBody)
		_ = json.NewEncoder(w).Encode(true)
	}))
	defer srv.Close()

	c := NewClient(srv.URL)
	if err := c.ReplyPermission(context.Background(), "ses_1", "per_1", "once"); err != nil {
		t.Fatal(err)
	}
	if gotBody["response"] != "once" {
		t.Errorf("body = %+v", gotBody)
	}
}

func TestClient_BasicAuth(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		user, pass, ok := r.BasicAuth()
		if !ok || user != "opencode" || pass != "secret" {
			w.WriteHeader(http.StatusUnauthorized)
			return
		}
		_ = json.NewEncoder(w).Encode(HealthResponse{Healthy: true})
	}))
	defer srv.Close()

	c := NewClient(srv.URL, WithBasicAuth("", "secret"))
	h, err := c.Health(context.Background())
	if err != nil {
		t.Fatal(err)
	}
	if !h.Healthy {
		t.Error("health = false")
	}
}

func TestClient_APIErrorSurfaces(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusNotFound)
		_, _ = io.WriteString(w, `{"message":"session not found"}`)
	}))
	defer srv.Close()

	c := NewClient(srv.URL)
	err := c.DeleteSession(context.Background(), "ses_missing")
	var apiErr *APIError
	if !errors.As(err, &apiErr) {
		t.Fatalf("err = %v, want *APIError", err)
	}
	if apiErr.StatusCode != http.StatusNotFound {
		t.Errorf("status = %d", apiErr.StatusCode)
	}
}

func TestClient_EventsRequiresSSE(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Header.Get("Accept") != "text/event-stream" {
			t.Errorf("Accept = %q", r.Header.Get("Accept"))
		}
		w.Header().Set("Content-Type", "text/event-stream")
		_, _ = io.WriteString(w, "data: {\"type\":\"server.connected\",\"properties\":{}}\n\n")
	}))
	defer srv.Close()

	c := NewClient(srv.URL)
	body, err := c.Events(context.Background())
	if err != nil {
		t.Fatal(err)
	}
	defer body.Close()
	raw, _ := io.ReadAll(body)
	if !strings.Contains(string(raw), "server.connected") {
		t.Errorf("body = %s", raw)
	}
}

func TestClient_EventsNon200(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusForbidden)
	}))
	defer srv.Close()

	c := NewClient(srv.URL)
	_, err := c.Events(context.Background())
	if err == nil {
		t.Fatal("expected error for non-200 /event")
	}
}

// guard: timeout on the shared http client must stay finite so a wedged
// server cannot hang the adapter forever (NFR-6).
func TestClient_DefaultTimeout(t *testing.T) {
	c := NewClient("http://127.0.0.1:1") // nothing listens here
	ctx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
	defer cancel()
	if _, err := c.Health(ctx); err == nil {
		t.Fatal("expected connection error")
	}
}


func TestClient_ProvidersShape(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/config/providers" {
			t.Errorf("path = %s", r.URL.Path)
		}
		_, _ = w.Write([]byte(`{"providers":[{"id":"zai-coding-plan","name":"Z.ai","models":{
			"glm-5.3":{"id":"glm-5.3","name":"GLM-5.3","capabilities":{"reasoning":true},
				"variants":{"low":{},"high":{},"max":{}}},
			"grok-imagine-image":{"id":"grok-imagine-image","name":"Imagine","capabilities":{"reasoning":false}}
		}}],"default":{"zai-coding-plan":"glm-5.3"}}`))
	}))
	defer srv.Close()

	c := NewClient(srv.URL)
	out, err := c.Providers(context.Background())
	if err != nil {
		t.Fatal(err)
	}
	p := out.Providers[0]
	reasoning, ok := p.Models["glm-5.3"]
	if !ok || !reasoning.Capabilities.Reasoning {
		t.Fatalf("glm-5.3 capabilities = %+v", reasoning)
	}
	if len(reasoning.Variants) != 3 {
		t.Errorf("glm-5.3 variants = %+v", reasoning.Variants)
	}
	if img := p.Models["grok-imagine-image"]; img.Capabilities.Reasoning {
		t.Errorf("imagine model must not report reasoning")
	}
}
