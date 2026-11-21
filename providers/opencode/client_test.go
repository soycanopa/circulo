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
	if err := c.Prompt(context.Background(), id, "hello"); err != nil {
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
