package opencode

// v2 surface of the OpenCode server API (docs/opencode-v2-migration.md).
// These methods live on the same Client (shared plumbing/auth) and wrap the
// `{location?, data: …}` response envelope v2 adds around every payload.

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
)

// dataEnvelope is the v2 response wrapper: every payload rides in `data`.
type dataEnvelope[T any] struct {
	Location *V2Location `json:"location,omitempty"`
	Data     T           `json:"data"`
}

// getV2 fetches path and unwraps the `data` envelope into out.
func getV2[T any](c *Client, ctx context.Context, path string, out *T) error {
	var env dataEnvelope[T]
	if err := c.get(ctx, path, &env); err != nil {
		return err
	}
	*out = env.Data
	return nil
}

// ServerInfoV2 calls GET /api/info — the v2 readiness probe.
func (c *Client) ServerInfoV2(ctx context.Context) (V2ServerInfo, error) {
	var out V2ServerInfo
	err := c.get(ctx, "/api/info", &out)
	return out, err
}

// LocationV2 calls GET /api/location; Directory is the resolved scope root.
func (c *Client) LocationV2(ctx context.Context) (V2Location, error) {
	var out V2Location
	err := c.get(ctx, "/api/location", &out)
	return out, err
}

// SessionsV2 calls GET /api/session.
func (c *Client) SessionsV2(ctx context.Context) ([]V2Session, error) {
	var out []V2Session
	err := getV2(c, ctx, "/api/session", &out)
	return out, err
}

// CreateSessionV2 calls POST /api/session (title/agent/model optional).
func (c *Client) CreateSessionV2(ctx context.Context, title string) (V2Session, error) {
	body := map[string]any{}
	if title != "" {
		body["title"] = title
	}
	var env dataEnvelope[V2Session]
	if err := c.post(ctx, "/api/session", body, &env); err != nil {
		return V2Session{}, err
	}
	return env.Data, nil
}

// RenameSessionV2 calls PATCH /api/session/{id}.
func (c *Client) RenameSessionV2(ctx context.Context, id, title string) error {
	return c.patch(ctx, "/api/session/"+id, map[string]any{"title": title}, nil)
}

// DeleteSessionV2 calls DELETE /api/session/{id}.
func (c *Client) DeleteSessionV2(ctx context.Context, id string) error {
	return c.do(ctx, http.MethodDelete, "/api/session/"+id, nil, nil)
}

// MessagesV2 calls GET /api/session/{id}/message (history hydration). The
// list carries full content inline; items are newest-first.
func (c *Client) MessagesV2(ctx context.Context, sessionID string, limit int) ([]V2Message, error) {
	path := "/api/session/" + sessionID + "/message"
	if limit > 0 {
		path = fmt.Sprintf("%s?limit=%d", path, limit)
	}
	var out []V2Message
	err := getV2(c, ctx, path, &out)
	return out, err
}

// PromptV2 sends the turn text (POST /api/session/{id}/prompt). The model is
// session-scoped in v2: call SetModelV2 first when the request selects one.
func (c *Client) PromptV2(ctx context.Context, sessionID, text string) error {
	body := map[string]any{"text": text}
	return c.post(ctx, "/api/session/"+sessionID+"/prompt", body, nil)
}

// SetModelV2 pins the session's model (POST /api/session/{id}/model).
func (c *Client) SetModelV2(ctx context.Context, sessionID, providerID, modelID string) error {
	body := map[string]any{
		"model": map[string]string{"id": modelID, "providerID": providerID},
	}
	return c.post(ctx, "/api/session/"+sessionID+"/model", body, nil)
}

// InterruptV2 aborts the running turn (POST /api/session/{id}/interrupt).
func (c *Client) InterruptV2(ctx context.Context, sessionID string) error {
	return c.post(ctx, "/api/session/"+sessionID+"/interrupt", map[string]any{}, nil)
}

// ReplyPermissionV2 answers a v2 permission request. The decision vocabulary
// (once|always|reject) is unchanged from v1; the body key and endpoint moved.
func (c *Client) ReplyPermissionV2(ctx context.Context, sessionID, requestID, decision string) error {
	path := fmt.Sprintf("/api/session/%s/permission/%s/reply", sessionID, requestID)
	return c.post(ctx, path, map[string]string{"decision": decision}, nil)
}

// AgentsV2 calls GET /api/agent (hidden agents included; caller filters).
func (c *Client) AgentsV2(ctx context.Context) ([]V2Agent, error) {
	var out []V2Agent
	err := getV2(c, ctx, "/api/agent", &out)
	return out, err
}

// ModelsV2 calls GET /api/model (enabled and disabled; caller filters).
func (c *Client) ModelsV2(ctx context.Context) ([]V2Model, error) {
	var out []V2Model
	err := getV2(c, ctx, "/api/model", &out)
	return out, err
}

// ProvidersV2 calls GET /api/provider.
func (c *Client) ProvidersV2(ctx context.Context) ([]V2Provider, error) {
	var out []V2Provider
	err := getV2(c, ctx, "/api/provider", &out)
	return out, err
}

// EventsV2 opens GET /api/event — same SSE framing as v1 (data lines +
// heartbeat comments), payload envelope parsed by the caller.
func (c *Client) EventsV2(ctx context.Context) (io.ReadCloser, error) {
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, c.url("/api/event"), nil)
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
		return nil, fmt.Errorf("opencode: /api/event returned %s", resp.Status)
	}
	return resp.Body, nil
}

// parseV2Event decodes one SSE data line into a V2Event.
func parseV2Event(raw []byte) (V2Event, error) {
	var env V2Event
	if err := json.Unmarshal(raw, &env); err != nil {
		return V2Event{}, fmt.Errorf("opencode: v2 event: %w", err)
	}
	return env, nil
}
