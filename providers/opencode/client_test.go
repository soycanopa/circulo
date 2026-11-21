package opencode

import (
	"context"
	"io"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"

	"github.com/soycanopa/circulo/driver"
)

func TestCreatePromptAndDelta(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		user, pass, ok := r.BasicAuth()
		if !ok || user != "opencode" || pass != "secret" {
			http.Error(w, "no", http.StatusUnauthorized)
			return
		}
		switch {
		case r.Method == http.MethodPost && r.URL.Path == "/api/session":
			body, _ := io.ReadAll(r.Body)
			if !strings.Contains(string(body), `"directory":"/work"`) {
				t.Errorf("body %s", body)
			}
			w.Header().Set("Content-Type", "application/json")
			io.WriteString(w, `{"data":{"id":"ses_test"}}`)
		case r.Method == http.MethodPost && r.URL.Path == "/api/session/ses_test/prompt":
			body, _ := io.ReadAll(r.Body)
			if !strings.Contains(string(body), `"text":"hello"`) {
				t.Errorf("prompt %s", body)
			}
			w.WriteHeader(http.StatusOK)
			io.WriteString(w, `{"data":{}}`)
		case r.URL.Path == "/api/event":
			w.Header().Set("Content-Type", "text/event-stream")
			io.WriteString(w, "data: {\"type\":\"server.connected\",\"data\":{}}\n\n")
			io.WriteString(w, ": heartbeat\n\n")
			io.WriteString(w, "data: {\"type\":\"session.text.delta\",\"data\":{\"sessionID\":\"ses_test\",\"assistantMessageID\":\"msg_1\",\"ordinal\":0,\"delta\":\"Hi\"}}\n\n")
			io.WriteString(w, "data: {\"type\":\"plugin.noise\",\"data\":{}}\n\n")
		default:
			http.NotFound(w, r)
		}
	}))
	defer srv.Close()

	c := &Client{BaseURL: srv.URL, Password: "secret"}
	id, err := c.CreateSession(context.Background(), "/work", "work")
	if err != nil || id != "ses_test" {
		t.Fatalf("create %q %v", id, err)
	}
	if err := c.Prompt(context.Background(), id, "hello", ""); err != nil {
		t.Fatal(err)
	}
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()
	var got []driver.Event
	err = c.stream(ctx, func(ev driver.Event) {
		got = append(got, ev)
		cancel()
	})
	if len(got) != 1 || got[0].Kind != driver.KindDelta || got[0].Text != "Hi" || got[0].MessageID != "msg_1" {
		t.Fatalf("events %#v err %v", got, err)
	}
}

func TestModelSteerAndAbort(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		switch {
		case r.Method == http.MethodGet && r.URL.Path == "/api/model":
			if r.URL.Query().Get("location[directory]") != "/work" || r.URL.Query().Get("types") != "chat" {
				t.Errorf("query %s", r.URL.RawQuery)
			}
			io.WriteString(w, `{"data":[
				{"id":"big-pickle","providerID":"opencode","name":"Big Pickle","enabled":true,"variants":[{"id":"low"},{"id":"high"}]},
				{"id":"hidden","providerID":"opencode","name":"Hidden","enabled":false,"variants":[]}
			]}`)
		case r.Method == http.MethodGet && r.URL.Path == "/api/model/default":
			io.WriteString(w, `{"data":{"id":"big-pickle","providerID":"opencode","name":"Big Pickle"}}`)
		case r.Method == http.MethodGet && r.URL.Path == "/api/session/ses_test":
			io.WriteString(w, `{"data":{"model":{"id":"big-pickle","providerID":"opencode","variant":"low"}}}`)
		case r.Method == http.MethodPost && r.URL.Path == "/api/session/ses_test/model":
			body, _ := io.ReadAll(r.Body)
			if !strings.Contains(string(body), `"variant":"high"`) || !strings.Contains(string(body), `"providerID":"opencode"`) {
				t.Errorf("model %s", body)
			}
			w.WriteHeader(http.StatusNoContent)
		case r.Method == http.MethodPost && r.URL.Path == "/api/session/ses_test/prompt":
			body, _ := io.ReadAll(r.Body)
			if !strings.Contains(string(body), `"delivery":"steer"`) {
				t.Errorf("prompt %s", body)
			}
			io.WriteString(w, `{"data":{}}`)
		case r.Method == http.MethodPost && r.URL.Path == "/api/session/ses_test/interrupt":
			w.WriteHeader(http.StatusOK)
			io.WriteString(w, `{"interrupted":true}`)
		default:
			http.NotFound(w, r)
		}
	}))
	defer srv.Close()
	c := &Client{BaseURL: srv.URL, Password: "secret"}
	ctx := context.Background()
	models, err := c.Models(ctx, "/work")
	if err != nil || len(models) != 1 || models[0].ID != "big-pickle" || len(models[0].Variants) != 2 {
		t.Fatalf("%#v %v", models, err)
	}
	def, err := c.DefaultModel(ctx, "/work")
	if err != nil || def.ID != "big-pickle" {
		t.Fatalf("%#v %v", def, err)
	}
	cur, err := c.SessionModel(ctx, "ses_test")
	if err != nil || cur.Variant != "low" {
		t.Fatalf("%#v %v", cur, err)
	}
	if err := c.SetModel(ctx, "ses_test", driver.Model{ID: "big-pickle", ProviderID: "opencode", Variant: "high"}); err != nil {
		t.Fatal(err)
	}
	if err := c.Prompt(ctx, "ses_test", "wait", "steer"); err != nil {
		t.Fatal(err)
	}
	if err := c.Abort(ctx, "ses_test"); err != nil {
		t.Fatal(err)
	}
}
