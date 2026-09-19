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

// Client speaks the OpenCode server API (v2 methods in client_v2.go). It is
// transport-only: no event translation, no process management (those live in
// translate_v2.go/adapter.go).
type Client struct {
	baseURL string
	http    *http.Client
	auth    *basicAuth // nil when the server needs no auth
}

type basicAuth struct{ user, pass string }

// ClientOption configures a Client.
type ClientOption func(*Client)

// WithBasicAuth sends Authorization: Basic on every request. v2 servers print
// their boot password and require Basic auth (user "opencode"); attach mode
// passes the user-configured credentials.
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
