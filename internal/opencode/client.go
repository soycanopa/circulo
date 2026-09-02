package opencode

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"time"
)

// Client speaks the OpenCode server API v1. It is transport-only: no event
// translation, no process management (those live in translate.go/adapter.go).
type Client struct {
	baseURL string
	http    *http.Client
	auth    *basicAuth // nil when no OPENCODE_SERVER_PASSWORD configured
}

type basicAuth struct{ user, pass string }

// ClientOption configures a Client.
type ClientOption func(*Client)

// WithBasicAuth sends Authorization: Basic on every request. Used when the
// server was started with OPENCODE_SERVER_PASSWORD (username defaults to
// "opencode", overridable via OPENCODE_SERVER_USERNAME).
func WithBasicAuth(user, pass string) ClientOption {
	return func(c *Client) {
		if user == "" {
			user = "opencode"
		}
		c.auth = &basicAuth{user: user, pass: pass}
	}
}

// WithHTTPClient overrides the underlying http.Client (tests inject one).
func WithHTTPClient(h *http.Client) ClientOption {
	return func(c *Client) { c.http = h }
}

// NewClient targets a running server, e.g. http://127.0.0.1:4096.
func NewClient(baseURL string, opts ...ClientOption) *Client {
	c := &Client{
		baseURL: baseURL,
		http:    &http.Client{Timeout: 30 * time.Second},
	}
	for _, opt := range opts {
		opt(c)
	}
	return c
}

// Health calls GET /global/health; used for readiness polling.
func (c *Client) Health(ctx context.Context) (HealthResponse, error) {
	var out HealthResponse
	err := c.get(ctx, "/global/health", &out)
	return out, err
}

// Path calls GET /path; Directory is the server's resolved scope root.
func (c *Client) Path(ctx context.Context) (PathResponse, error) {
	var out PathResponse
	err := c.get(ctx, "/path", &out)
	return out, err
}

// Sessions calls GET /session.
func (c *Client) Sessions(ctx context.Context) ([]Session, error) {
	var out []Session
	err := c.get(ctx, "/session", &out)
	return out, err
}

// CreateSession calls POST /session.
func (c *Client) CreateSession(ctx context.Context, title string) (Session, error) {
	body := map[string]string{}
	if title != "" {
		body["title"] = title
	}
	var out Session
	err := c.post(ctx, "/session", body, &out)
	return out, err
}

// RenameSession calls PATCH /session/{id}.
func (c *Client) RenameSession(ctx context.Context, id, title string) error {
	return c.patch(ctx, "/session/"+id, map[string]string{"title": title}, nil)
}

// DeleteSession calls DELETE /session/{id}.
func (c *Client) DeleteSession(ctx context.Context, id string) error {
	return c.do(ctx, http.MethodDelete, "/session/"+id, nil, nil)
}

// Messages calls GET /session/{id}/message (history hydration).
func (c *Client) Messages(ctx context.Context, sessionID string, limit int) ([]MessagesPage, error) {
	path := "/session/" + sessionID + "/message"
	if limit > 0 {
		path = fmt.Sprintf("%s?limit=%d", path, limit)
	}
	var out []MessagesPage
	err := c.get(ctx, path, &out)
	return out, err
}

// Prompt sends the turn and returns immediately (POST /session/{id}/prompt_async
// → 204). Content arrives over the event stream.
func (c *Client) Prompt(ctx context.Context, sessionID string, req PromptInput) error {
	body := map[string]any{
		"parts": []map[string]string{{"type": "text", "text": req.Text}},
	}
	if req.Provider != "" || req.Model != "" {
		body["model"] = map[string]string{"providerID": req.Provider, "modelID": req.Model}
	}
	if req.Agent != "" {
		body["agent"] = req.Agent
	}
	return c.post(ctx, "/session/"+sessionID+"/prompt_async", body, nil)
}

// PromptInput is the neutral prompt the adapter forwards.
type PromptInput struct {
	Text     string
	Agent    string
	Provider string
	Model    string
}

// Abort calls POST /session/{id}/abort.
func (c *Client) Abort(ctx context.Context, sessionID string) error {
	return c.post(ctx, "/session/"+sessionID+"/abort", map[string]any{}, nil)
}

// ReplyPermission answers a permission.asked with once|always|reject.
func (c *Client) ReplyPermission(ctx context.Context, sessionID, permissionID, response string) error {
	path := fmt.Sprintf("/session/%s/permissions/%s", sessionID, permissionID)
	return c.post(ctx, path, map[string]string{"response": response}, nil)
}

// Agents calls GET /agent.
func (c *Client) Agents(ctx context.Context) ([]AgentInfo, error) {
	var out []AgentInfo
	err := c.get(ctx, "/agent", &out)
	return out, err
}

// Providers calls GET /config/providers.
func (c *Client) Providers(ctx context.Context) (ProvidersResponse, error) {
	var out ProvidersResponse
	err := c.get(ctx, "/config/providers", &out)
	return out, err
}

// Events opens GET /event and hands the raw body to consume. The returned
// cancel func must be called to release the connection.
func (c *Client) Events(ctx context.Context) (io.ReadCloser, error) {
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, c.url("/event"), nil)
	if err != nil {
		return nil, err
	}
	req.Header.Set("Accept", "text/event-stream")
	if c.auth != nil {
		req.SetBasicAuth(c.auth.user, c.auth.pass)
	}
	resp, err := c.http.Do(req)
	if err != nil {
		return nil, err
	}
	if resp.StatusCode != http.StatusOK {
		resp.Body.Close()
		return nil, fmt.Errorf("opencode: /event returned %s", resp.Status)
	}
	return resp.Body, nil
}

func (c *Client) url(path string) string { return c.baseURL + path }

func (c *Client) get(ctx context.Context, path string, out any) error {
	return c.do(ctx, http.MethodGet, path, nil, out)
}

func (c *Client) post(ctx context.Context, path string, body, out any) error {
	return c.do(ctx, http.MethodPost, path, body, out)
}

func (c *Client) patch(ctx context.Context, path string, body, out any) error {
	return c.do(ctx, http.MethodPatch, path, body, out)
}

func (c *Client) do(ctx context.Context, method, path string, body, out any) error {
	var reader io.Reader
	if body != nil {
		raw, err := json.Marshal(body)
		if err != nil {
			return fmt.Errorf("opencode: encode %s %s: %w", method, path, err)
		}
		reader = bytes.NewReader(raw)
	}
	req, err := http.NewRequestWithContext(ctx, method, c.url(path), reader)
	if err != nil {
		return err
	}
	if body != nil {
		req.Header.Set("Content-Type", "application/json")
	}
	if c.auth != nil {
		req.SetBasicAuth(c.auth.user, c.auth.pass)
	}
	resp, err := c.http.Do(req)
	if err != nil {
		return fmt.Errorf("opencode: %s %s: %w", method, path, err)
	}
	defer resp.Body.Close()
	if resp.StatusCode >= 300 {
		snippet, _ := io.ReadAll(io.LimitReader(resp.Body, 512))
		return &APIError{
			Method:     method,
			Path:       path,
			StatusCode: resp.StatusCode,
			Body:       string(snippet),
		}
	}
	if out != nil {
		if err := json.NewDecoder(io.LimitReader(resp.Body, 32<<20)).Decode(out); err != nil {
			return fmt.Errorf("opencode: decode %s %s: %w", method, path, err)
		}
	}
	return nil
}

// APIError is a non-2xx from the OpenCode server.
type APIError struct {
	Method     string
	Path       string
	StatusCode int
	Body       string
}

func (e *APIError) Error() string {
	if e.Body != "" {
		return fmt.Sprintf("opencode: %s %s: %d: %.200s", e.Method, e.Path, e.StatusCode, e.Body)
	}
	return fmt.Sprintf("opencode: %s %s: %d", e.Method, e.Path, e.StatusCode)
}
