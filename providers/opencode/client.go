// Package opencode speaks the OpenCode v2 HTTP API.
// It does not import the core or the UI.
//
// Endpoints, from the OpenAPI document served by `opencode serve` v2:
//
//	POST /api/session                         create a session at a directory
//	POST /api/session/{sessionID}/prompt     body {"text": "..."}
//	GET  /api/event                           SSE, each frame is `data: {event json}`
//
// The server uses Basic auth. The username is opencode and the password is
// the token `opencode serve` prints as "server password".
package opencode

import (
	"bufio"
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"strings"
	"time"

	"github.com/soycanopa/circulo/driver"
)

// Client is one OpenCode server.
type Client struct {
	BaseURL  string
	User     string
	Password string
	HTTP     *http.Client
}

func (c *Client) client() *http.Client {
	if c.HTTP != nil {
		return c.HTTP
	}
	return http.DefaultClient
}

func (c *Client) user() string {
	if c.User != "" {
		return c.User
	}
	return "opencode"
}

// CreateSession creates a session whose files live in directory.
func (c *Client) CreateSession(ctx context.Context, directory, title string) (string, error) {
	var out struct {
		Data struct {
			ID string `json:"id"`
		} `json:"data"`
	}
	err := c.do(ctx, http.MethodPost, "/api/session", map[string]any{
		"title":    title,
		"location": map[string]string{"directory": directory},
	}, &out)
	if err != nil {
		return "", err
	}
	if out.Data.ID == "" {
		return "", fmt.Errorf("opencode: session response has no id")
	}
	return out.Data.ID, nil
}

// Prompt admits one user message. delivery is empty, "steer", or "queue".
func (c *Client) Prompt(ctx context.Context, sessionID, text, delivery string) error {
	body := map[string]any{"text": text}
	if delivery != "" {
		body["delivery"] = delivery
	}
	return c.do(ctx, http.MethodPost, "/api/session/"+url.PathEscape(sessionID)+"/prompt", body, nil)
}

// Models lists enabled chat models for a project directory.
func (c *Client) Models(ctx context.Context, directory string) ([]driver.Model, error) {
	var out struct {
		Data []struct {
			ID         string `json:"id"`
			ProviderID string `json:"providerID"`
			Name       string `json:"name"`
			Enabled    bool   `json:"enabled"`
			Variants   []struct {
				ID string `json:"id"`
			} `json:"variants"`
		} `json:"data"`
	}
	if err := c.do(ctx, http.MethodGet, "/api/model?"+locationQuery(directory)+"&types=chat", nil, &out); err != nil {
		return nil, err
	}
	var models []driver.Model
	for _, m := range out.Data {
		if !m.Enabled {
			continue
		}
		item := driver.Model{ID: m.ID, ProviderID: m.ProviderID, Name: m.Name}
		for _, v := range m.Variants {
			if v.ID != "" {
				item.Variants = append(item.Variants, v.ID)
			}
		}
		models = append(models, item)
	}
	return models, nil
}

// DefaultModel is the model OpenCode would use for new work in directory.
func (c *Client) DefaultModel(ctx context.Context, directory string) (driver.Model, error) {
	var out struct {
		Data *struct {
			ID         string `json:"id"`
			ProviderID string `json:"providerID"`
			Name       string `json:"name"`
			Variants   []struct {
				ID string `json:"id"`
			} `json:"variants"`
		} `json:"data"`
	}
	if err := c.do(ctx, http.MethodGet, "/api/model/default?"+locationQuery(directory), nil, &out); err != nil {
		return driver.Model{}, err
	}
	if out.Data == nil {
		return driver.Model{}, nil
	}
	item := driver.Model{ID: out.Data.ID, ProviderID: out.Data.ProviderID, Name: out.Data.Name}
	for _, v := range out.Data.Variants {
		if v.ID != "" {
			item.Variants = append(item.Variants, v.ID)
		}
	}
	return item, nil
}

// SessionModel reads the model currently selected on a session.
func (c *Client) SessionModel(ctx context.Context, sessionID string) (driver.Model, error) {
	var out struct {
		Data struct {
			Model struct {
				ID         string `json:"id"`
				ProviderID string `json:"providerID"`
				Variant    string `json:"variant"`
			} `json:"model"`
		} `json:"data"`
	}
	if err := c.do(ctx, http.MethodGet, "/api/session/"+url.PathEscape(sessionID), nil, &out); err != nil {
		return driver.Model{}, err
	}
	m := out.Data.Model
	return driver.Model{ID: m.ID, ProviderID: m.ProviderID, Variant: m.Variant}, nil
}

// SetModel switches the session's model and reasoning effort.
func (c *Client) SetModel(ctx context.Context, sessionID string, model driver.Model) error {
	body := map[string]any{"id": model.ID, "providerID": model.ProviderID}
	if model.Variant != "" {
		body["variant"] = model.Variant
	}
	return c.do(ctx, http.MethodPost, "/api/session/"+url.PathEscape(sessionID)+"/model", body, nil)
}

// Abort interrupts the session's current turn.
func (c *Client) Abort(ctx context.Context, sessionID string) error {
	return c.do(ctx, http.MethodPost, "/api/session/"+url.PathEscape(sessionID)+"/interrupt", nil, nil)
}

func locationQuery(directory string) string {
	q := url.Values{}
	q.Set("location[directory]", directory)
	return q.Encode()
}

// Watch reads the event stream until ctx ends, reconnecting after the server closes it.
// Unknown event types are ignored.
func (c *Client) Watch(ctx context.Context, fn func(driver.Event)) error {
	for {
		if err := ctx.Err(); err != nil {
			return err
		}
		err := c.stream(ctx, fn)
		if ctx.Err() != nil {
			return ctx.Err()
		}
		if err != nil && err != io.EOF {
			return err
		}
		t := time.NewTimer(200 * time.Millisecond)
		select {
		case <-ctx.Done():
			t.Stop()
			return ctx.Err()
		case <-t.C:
		}
	}
}

func (c *Client) stream(ctx context.Context, fn func(driver.Event)) error {
	q := url.Values{}
	for _, typ := range []string{
		"session.text.delta",
		"session.text.ended",
		"session.execution.succeeded",
		"session.execution.failed",
		"session.execution.interrupted",
		"session.model.selected",
	} {
		q.Add("types", typ)
	}
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, c.BaseURL+"/api/event?"+q.Encode(), nil)
	if err != nil {
		return err
	}
	req.SetBasicAuth(c.user(), c.Password)
	resp, err := c.client().Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()
	if resp.StatusCode != http.StatusOK {
		b, _ := io.ReadAll(io.LimitReader(resp.Body, 512))
		return fmt.Errorf("opencode: event stream %s: %s", resp.Status, bytes.TrimSpace(b))
	}
	return readSSE(resp.Body, func(payload []byte) {
		ev, ok := translate(payload)
		if ok {
			fn(ev)
		}
	})
}

func (c *Client) do(ctx context.Context, method, path string, body any, dest any) error {
	var buf bytes.Buffer
	if body != nil {
		if err := json.NewEncoder(&buf).Encode(body); err != nil {
			return err
		}
	}
	req, err := http.NewRequestWithContext(ctx, method, c.BaseURL+path, &buf)
	if err != nil {
		return err
	}
	req.SetBasicAuth(c.user(), c.Password)
	if body != nil {
		req.Header.Set("Content-Type", "application/json")
	}
	resp, err := c.client().Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()
	b, err := io.ReadAll(io.LimitReader(resp.Body, 1<<20))
	if err != nil {
		return err
	}
	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return fmt.Errorf("opencode: %s %s: %s", method, path, bytes.TrimSpace(b))
	}
	if dest == nil || len(bytes.TrimSpace(b)) == 0 {
		return nil
	}
	return json.Unmarshal(b, dest)
}

func readSSE(r io.Reader, fn func([]byte)) error {
	sc := bufio.NewScanner(r)
	sc.Buffer(make([]byte, 0, 64*1024), 1<<20)
	var data strings.Builder
	flush := func() {
		if data.Len() == 0 {
			return
		}
		fn([]byte(data.String()))
		data.Reset()
	}
	for sc.Scan() {
		line := sc.Text()
		switch {
		case line == "":
			flush()
		case strings.HasPrefix(line, ":"):
		case strings.HasPrefix(line, "data:"):
			if data.Len() > 0 {
				data.WriteByte('\n')
			}
			data.WriteString(strings.TrimPrefix(strings.TrimPrefix(line, "data:"), " "))
		}
	}
	flush()
	if err := sc.Err(); err != nil {
		return err
	}
	return io.EOF
}

func translate(payload []byte) (driver.Event, bool) {
	var ev struct {
		Type string `json:"type"`
		Data struct {
			SessionID          string `json:"sessionID"`
			AssistantMessageID string `json:"assistantMessageID"`
			Delta              string `json:"delta"`
			Text               string `json:"text"`
			Error              any    `json:"error"`
			Reason             string `json:"reason"`
			Model              struct {
				ID         string `json:"id"`
				ProviderID string `json:"providerID"`
				Variant    string `json:"variant"`
			} `json:"model"`
		} `json:"data"`
	}
	if err := json.Unmarshal(payload, &ev); err != nil {
		return driver.Event{}, false
	}
	switch ev.Type {
	case "session.text.delta":
		return driver.Event{Kind: driver.KindDelta, SessionID: ev.Data.SessionID, MessageID: ev.Data.AssistantMessageID, Text: ev.Data.Delta}, true
	case "session.text.ended":
		return driver.Event{Kind: driver.KindEnded, SessionID: ev.Data.SessionID, MessageID: ev.Data.AssistantMessageID, Text: ev.Data.Text}, true
	case "session.execution.succeeded":
		return driver.Event{Kind: driver.KindDone, SessionID: ev.Data.SessionID}, true
	case "session.execution.failed":
		return driver.Event{Kind: driver.KindFailed, SessionID: ev.Data.SessionID, Text: fmt.Sprint(ev.Data.Error)}, true
	case "session.execution.interrupted":
		return driver.Event{Kind: driver.KindFailed, SessionID: ev.Data.SessionID, Text: "interrupted: " + ev.Data.Reason}, true
	case "session.model.selected":
		return driver.Event{
			Kind: driver.KindModel, SessionID: ev.Data.SessionID,
			ModelID: ev.Data.Model.ID, ProviderID: ev.Data.Model.ProviderID, Variant: ev.Data.Model.Variant,
		}, true
	default:
		return driver.Event{}, false
	}
}
