package opencode

import "encoding/json"

// Wire types for the OpenCode v2 API (docs/opencode-v2-migration.md). Shapes
// verified against a live 2.0.8 server via /openapi.json + captures; keep
// them in sync with the spec whenever the pinned version changes.

// V2ServerInfo is GET /api/info — v2's readiness probe (v1: /global/health,
// which now serves the SPA fallback for unknown paths).
type V2ServerInfo struct {
	Version string   `json:"version"`
	PID     int      `json:"pid"`
	URLs    []string `json:"urls"`
}

// V2Location is GET /api/location; Directory is the resolved scope root the
// server is rooted at (the key used to scope sessions to this project).
type V2Location struct {
	Directory string `json:"directory"`
	Project   struct {
		ID        string `json:"id"`
		Directory string `json:"directory"`
		Canonical string `json:"canonical"`
	} `json:"project"`
}

// V2Session is one item of GET /api/session.
type V2Session struct {
	ID        string `json:"id"`
	ProjectID string `json:"projectID"`
	Title     string `json:"title"`
	Time      struct {
		Created int64 `json:"created"`
		Updated int64 `json:"updated"`
	} `json:"time"`
	Location struct {
		Directory string `json:"directory"`
	} `json:"location"`
	Model struct {
		ID         string `json:"id"`
		ProviderID string `json:"providerID"`
		Variant    string `json:"variant"`
	} `json:"model"`
	Cost   float64       `json:"cost"`
	Tokens *V2TokenUsage `json:"tokens,omitempty"`
}

// V2TokenUsage is the shared token accounting block.
type V2TokenUsage struct {
	Input     int64 `json:"input"`
	Output    int64 `json:"output"`
	Reasoning int64 `json:"reasoning"`
	Cache     struct {
		Read  int64 `json:"read"`
		Write int64 `json:"write"`
	} `json:"cache"`
}

// V2Model is one item of GET /api/model. v2 has no capabilities.reasoning:
// a model supports effort selection when Variants is non-empty.
type V2Model struct {
	ID         string `json:"id"`
	ModelID    string `json:"modelID"`
	ProviderID string `json:"providerID"`
	Family     string `json:"family,omitempty"`
	Name       string `json:"name,omitempty"`
	Capabilities struct {
		Tools bool     `json:"tools"`
		Input []string `json:"input"`
		Output []string `json:"output"`
	} `json:"capabilities"`
	Variants []struct {
		ID string `json:"id"`
	} `json:"variants,omitempty"`
	Enabled bool `json:"enabled"`
}

// V2Provider is one item of GET /api/provider.
type V2Provider struct {
	ID   string `json:"id"`
	Name string `json:"name,omitempty"`
}

// V2Agent is one item of GET /api/agent (v2 keys agents by `id`, not name).
type V2Agent struct {
	ID          string `json:"id"`
	Name        string `json:"name,omitempty"`
	Description string `json:"description,omitempty"`
	Mode        string `json:"mode,omitempty"`
	Hidden      bool   `json:"hidden,omitempty"`
}

// V2Message is one item of GET /api/session/{id}/message. The list carries
// full content inline (no per-message detail round-trip needed). Items are
// newest-first and interleaved with `idle`/`system` markers.
type V2Message struct {
	ID   string `json:"id"`
	Time struct {
		Created   int64 `json:"created"`
		Streamed  int64 `json:"streamed,omitempty"`
		Completed int64 `json:"completed,omitempty"`
	} `json:"time"`
	Type  string `json:"type"` // user|assistant|system|idle
	Text  string `json:"text,omitempty"`
	Agent string `json:"agent,omitempty"`
	Model struct {
		ID         string `json:"id"`
		ProviderID string `json:"providerID"`
	} `json:"model,omitempty"`
	Content []V2Content `json:"content,omitempty"`
	Finish  string      `json:"finish,omitempty"`
	Cost    float64     `json:"cost,omitempty"`
	Tokens  *V2TokenUsage `json:"tokens,omitempty"`
	Outcome string      `json:"outcome,omitempty"`
}

// V2Content is one entry of an assistant message's content: text, reasoning
// or tool (tool entries carry their full state inline).
type V2Content struct {
	Type string `json:"type"` // text|reasoning|tool
	Text string `json:"text,omitempty"`
	ID   string `json:"id,omitempty"`
	Name string `json:"name,omitempty"`
	State struct {
		Status string          `json:"status"`
		Input  json.RawMessage `json:"input,omitempty"`
		Output string          `json:"output,omitempty"`
		Error  string          `json:"error,omitempty"`
	} `json:"state,omitempty"`
	Time struct {
		Created   int64 `json:"created"`
		Completed int64 `json:"completed,omitempty"`
	} `json:"time,omitempty"`
}

// V2Event is one frame of GET /api/event. The event name travels in `type`
// and the payload is the raw JSON in `data` (parsed per type by translate).
type V2Event struct {
	ID       string          `json:"id"`
	Type     string          `json:"type"`
	Data     json.RawMessage `json:"data,omitempty"`
	Created  int64           `json:"created,omitempty"`
}
