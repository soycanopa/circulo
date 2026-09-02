package opencode

import (
	"encoding/json"
	"fmt"
)

// Wire types for the OpenCode server API v1, transcribed from the OpenAPI 3.1
// spec served by the server itself (GET /doc), verified against 1.18.25.
// Only the fields circuloGo consumes are declared; decoding is tolerant so a
// server adding fields never breaks the adapter.

// Envelope is the /event SSE payload:
// {"id":"evt_…","type":"message.part.updated","properties":{…}}.
type Envelope struct {
	ID         string          `json:"id,omitempty"`
	Type       string          `json:"type"`
	Properties json.RawMessage `json:"properties"`
}

// DecodeEvent parses one raw SSE data frame. Unknown event types decode
// successfully (the translator decides what to skip) — this is the forward
// compatibility rule from docs/trd.md §10.3.
func DecodeEvent(frame Frame) (Envelope, error) {
	var env Envelope
	if err := json.Unmarshal(frame, &env); err != nil {
		return Envelope{}, fmt.Errorf("opencode: bad event frame: %w", err)
	}
	if env.Type == "" {
		return Envelope{}, fmt.Errorf("opencode: event frame without type: %.80s", frame)
	}
	return env, nil
}

// Session (GET /session, session.created/updated events properties.info).
type Session struct {
	ID        string `json:"id"`
	Title     string `json:"title"`
	Directory string `json:"directory,omitempty"`
	ProjectID string `json:"projectID,omitempty"`
	Time      struct {
		Created int64 `json:"created"`
		Updated int64 `json:"updated"`
	} `json:"time"`
}

// EventPropertiesSession is session.created/updated/deleted properties.
type EventPropertiesSession struct {
	SessionID string `json:"sessionID"`
	Info      Session `json:"info"`
}

// EventPropertiesSessionStatus is session.status properties. Status.Type is
// idle | busy | retry; Retry carries the provider backoff details (observed
// live with a real 429 plan restriction).
type EventPropertiesSessionStatus struct {
	SessionID string `json:"sessionID"`
	Status    struct {
		Type    string `json:"type"`
		Attempt int    `json:"attempt,omitempty"`
		Message string `json:"message,omitempty"`
		Next    int64  `json:"next,omitempty"` // unix millis
	} `json:"status"`
}

// Message is UserMessage | AssistantMessage. Only assistant carries the
// accounting fields; decode tolerates their absence on user messages.
type Message struct {
	ID        string `json:"id"`
	SessionID string `json:"sessionID"`
	Role      string `json:"role"`
	Time      struct {
		Created   int64 `json:"created"`
		Completed int64 `json:"completed,omitempty"`
	} `json:"time"`
	ProviderID string `json:"providerID,omitempty"`
	ModelID    string `json:"modelID,omitempty"`
	Agent      string `json:"agent,omitempty"`
	Mode       string `json:"mode,omitempty"`
	Cost       float64 `json:"cost,omitempty"`
	Tokens     *struct {
		Total     int64 `json:"total,omitempty"`
		Input     int64 `json:"input"`
		Output    int64 `json:"output"`
		Reasoning int64 `json:"reasoning"`
		Cache     struct {
			Read  int64 `json:"read"`
			Write int64 `json:"write"`
		} `json:"cache"`
	} `json:"tokens,omitempty"`
	Finish string `json:"finish,omitempty"`
	Error  *struct {
		Name string `json:"name,omitempty"`
		Data struct {
			Message string `json:"message,omitempty"`
		} `json:"data,omitempty"`
	} `json:"error,omitempty"`
}

// EventPropertiesMessage is message.updated properties.
type EventPropertiesMessage struct {
	Info Message `json:"info"`
}

// ToolState is the tool part state union (pending|running|completed|error).
// Status strings come straight from the server.
type ToolState struct {
	Status   string          `json:"status"`
	Input    json.RawMessage `json:"input,omitempty"`
	Title    string          `json:"title,omitempty"`
	Output   string          `json:"output,omitempty"`
	Error    string          `json:"error,omitempty"`
	Metadata json.RawMessage `json:"metadata,omitempty"`
	Time     *struct {
		Start int64 `json:"start"`
		End   int64 `json:"end,omitempty"`
	} `json:"time,omitempty"`
}

// Part is the OpenCode part union (flat by type discriminator).
// snapshot/retry/compaction parts decode but the translator drops them (v0).
type Part struct {
	ID        string `json:"id"`
	SessionID string `json:"sessionID,omitempty"`
	MessageID string `json:"messageID,omitempty"`
	Type      string `json:"type"`

	// text, reasoning
	Text      string `json:"text,omitempty"`
	Synthetic bool   `json:"synthetic,omitempty"`
	Time      *struct {
		Start int64 `json:"start,omitempty"`
		End   int64 `json:"end,omitempty"`
	} `json:"time,omitempty"`

	// tool
	CallID string    `json:"callID,omitempty"`
	Tool   string    `json:"tool,omitempty"`
	State  ToolState `json:"state,omitempty"`

	// step-finish
	Reason string `json:"reason,omitempty"`
	Cost   float64 `json:"cost,omitempty"`
	Tokens *struct {
		Total     int64 `json:"total,omitempty"`
		Input     int64 `json:"input"`
		Output    int64 `json:"output"`
		Reasoning int64 `json:"reasoning"`
		Cache     struct {
			Read  int64 `json:"read"`
			Write int64 `json:"write"`
		} `json:"cache"`
	} `json:"tokens,omitempty"`

	// patch
	Hash  string   `json:"hash,omitempty"`
	Files []string `json:"files,omitempty"`

	// agent
	Name string `json:"name,omitempty"`

	// subtask
	Prompt      string `json:"prompt,omitempty"`
	Description string `json:"description,omitempty"`
	Agent       string `json:"agent,omitempty"`

	// file
	Mime string `json:"mime,omitempty"`
	URL  string `json:"url,omitempty"`
	Source *struct {
		Type string `json:"type"`
		Path string `json:"path,omitempty"`
	} `json:"source,omitempty"`
}

// EventPropertiesPartUpdated is message.part.updated properties. Part is the
// full replacement; Delta is an optional incremental hint.
type EventPropertiesPartUpdated struct {
	Part  Part    `json:"part"`
	Delta *string `json:"delta,omitempty"`
}

// EventPropertiesPartDelta is message.part.delta properties.
type EventPropertiesPartDelta struct {
	SessionID string `json:"sessionID"`
	MessageID string `json:"messageID"`
	PartID    string `json:"partID"`
	Field     string `json:"field"` // "text" | "reasoning" | …
	Delta     string `json:"delta"`
}

// PermissionRequest is the permission.asked data payload (verified against the
// 1.18.25 OpenAPI: field is `permission` for the kind, `patterns` is a list,
// `tool.callID` links the pending tool part).
type PermissionRequest struct {
	ID         string          `json:"id"`
	SessionID  string          `json:"sessionID"`
	Permission string          `json:"permission"` // bash|edit|webfetch|…
	Patterns   []string        `json:"patterns,omitempty"`
	Metadata   json.RawMessage `json:"metadata,omitempty"`
	Always     []string        `json:"always,omitempty"`
	Tool       *struct {
		MessageID string `json:"messageID"`
		CallID    string `json:"callID"`
	} `json:"tool,omitempty"`
}

// EventPropertiesPermissionAsked is permission.asked properties.
type EventPropertiesPermissionAsked struct {
	ID   string            `json:"id"`
	Data PermissionRequest `json:"data"`
}

// EventPropertiesPermissionReplied is permission.replied properties.
type EventPropertiesPermissionReplied struct {
	Data struct {
		SessionID  string `json:"sessionID"`
		RequestID  string `json:"requestID"`
		Reply      string `json:"reply"`
	} `json:"data"`
}

// EventPropertiesSessionError is session.error properties.
type EventPropertiesSessionError struct {
	SessionID string `json:"sessionID,omitempty"`
	Error     *struct {
		Name string `json:"name"`
		Data struct {
			Message string `json:"message,omitempty"`
		} `json:"data"`
	} `json:"error"`
}

// MessagesPage is one element of GET /session/{id}/message.
type MessagesPage struct {
	Info  Message `json:"info"`
	Parts []Part  `json:"parts"`
}

// AgentInfo is an element of GET /agent.
type AgentInfo struct {
	Name        string `json:"name"`
	Description string `json:"description,omitempty"`
	Mode        string `json:"mode,omitempty"`
	Hidden      bool   `json:"hidden,omitempty"`
}

// ProvidersResponse is GET /config/providers.
type ProvidersResponse struct {
	Providers []struct {
		ID     string `json:"id"`
		Name   string `json:"name,omitempty"`
		Models map[string]struct {
			ID   string `json:"id"`
			Name string `json:"name,omitempty"`
		} `json:"models"`
	} `json:"providers"`
	Default map[string]string `json:"default,omitempty"`
}

// HealthResponse is GET /global/health.
type HealthResponse struct {
	Healthy bool   `json:"healthy"`
	Version string `json:"version"`
}

// PathResponse is GET /path. Directory is the resolved root the server is
// scoped to (e.g. /tmp/x reported as /private/tmp/x on macOS) — the key used
// to scope sessions to this project (Session.Directory matches its format).
type PathResponse struct {
	Worktree  string `json:"worktree,omitempty"`
	Directory string `json:"directory"`
}
