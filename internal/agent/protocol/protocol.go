// Package protocol is the neutral wire contract between the Go backend and the
// React frontend. It is the ONLY shared vocabulary: agent adapters translate
// their vendor events into these types, the UI consumes nothing else.
//
// JSON encoding is camelCase to mirror frontend/src/lib/agent/protocol.ts.
// Any struct change here must land in the same commit as its TS mirror
// (AGENTS.md: Project map).
package protocol

import "encoding/json"

// Event types carried by Envelope.Type.
const (
	EventAdapterStatus      = "adapter.status"
	EventSessionUpdated     = "session.updated"
	EventSessionStatus      = "session.status"
	EventSessionRemoved     = "session.removed"
	EventMessageUpdated     = "message.updated"
	EventPartUpdated        = "part.updated"
	EventPartDelta          = "part.delta"
	EventPermissionRequest  = "permission.request"
	EventPermissionResolved = "permission.resolved"
	EventSessionError       = "session.error"
)

// Adapter states (EventAdapterStatus).
const (
	AdapterStarting = "starting"
	AdapterRunning  = "running"
	AdapterStopped  = "stopped"
	AdapterError    = "error"
)

// Session statuses (EventSessionStatus).
const (
	StatusIdle  = "idle"
	StatusBusy  = "busy"
	StatusRetry = "retry"
)

// Part types (Part.Type).
const (
	PartText       = "text"
	PartReasoning  = "reasoning"
	PartTool       = "tool"
	PartStepStart  = "step-start"
	PartStepFinish = "step-finish"
	PartPatch      = "patch"
	PartAgent      = "agent"
	PartSubtask    = "subtask"
	PartFile       = "file"
)

// Tool states (ToolState.Status), mirroring OpenCode pending/running/completed/error.
const (
	ToolPending   = "pending"
	ToolRunning   = "running"
	ToolCompleted = "completed"
	ToolError     = "error"
)

// Message roles.
const (
	RoleUser      = "user"
	RoleAssistant = "assistant"
)

// Permission responses accepted by ReplyPermission.
const (
	PermissionOnce   = "once"
	PermissionAlways = "always"
	PermissionReject = "reject"
)

// Envelope is the top-level frame written to /agent/sse.
type Envelope struct {
	Type    string          `json:"type"`
	Payload json.RawMessage `json:"payload"`
}

// NewEnvelope marshals payload and wraps it. Marshal errors are programmer
// errors (protocol types are all JSON-safe); they panic in NewEnvelope's
// contract via json.Marshal returning them — callers treat Envelope as
// already-validated, so we surface the error instead of hiding it.
func NewEnvelope(typ string, payload any) (Envelope, error) {
	raw, err := json.Marshal(payload)
	if err != nil {
		return Envelope{}, err
	}
	return Envelope{Type: typ, Payload: raw}, nil
}

// AdapterStatus is the lifecycle of one project's agent server.
type AdapterStatus struct {
	ProjectID string `json:"projectID"`
	State     string `json:"state"` // starting|running|stopped|error
	Detail    string `json:"detail,omitempty"`
}

// Session is the sidebar-level view of an agent session.
type Session struct {
	ID          string `json:"id"`
	Title       string `json:"title"`
	Directory   string `json:"directory,omitempty"`
	TimeCreated int64  `json:"timeCreated"` // unix millis
	TimeUpdated int64  `json:"timeUpdated"` // unix millis
}

// RetryInfo accompanies StatusRetry.
type RetryInfo struct {
	Attempt int    `json:"attempt"`
	Message string `json:"message"`
	NextAt  int64  `json:"nextAt"` // unix millis, best-effort
}

// SessionStatus reports whether a session is working.
type SessionStatus struct {
	ProjectID string     `json:"projectID"`
	SessionID string     `json:"sessionID"`
	Status    string     `json:"status"` // idle|busy|retry
	Retry     *RetryInfo `json:"retry,omitempty"`
}

// TokenUsage is flat (no nested cache object) so the TS mirror stays simple.
type TokenUsage struct {
	Input     int64 `json:"input"`
	Output    int64 `json:"output"`
	Reasoning int64 `json:"reasoning"`
	CacheRead int64 `json:"cacheRead"`
	CacheWrite int64 `json:"cacheWrite"`
}

// MessageInfo is message metadata (identity + accounting). Content lives in Parts.
type MessageInfo struct {
	ID        string      `json:"id"`
	SessionID string      `json:"sessionID"`
	Role      string      `json:"role"`
	Created   int64       `json:"created"`  // unix millis
	Completed int64       `json:"completed"` // unix millis, 0 while streaming
	Provider  string      `json:"provider,omitempty"`
	Model     string      `json:"model,omitempty"`
	Agent     string      `json:"agent,omitempty"`
	Cost      float64     `json:"cost,omitempty"`
	Tokens    *TokenUsage `json:"tokens,omitempty"`
	Finish    string      `json:"finish,omitempty"`
	ErrorName string      `json:"errorName,omitempty"`
	ErrorMsg  string      `json:"errorMsg,omitempty"`
}

// MessageUpdated fires on message creation/metadata change (not content).
type MessageUpdated struct {
	ProjectID string      `json:"projectID"`
	SessionID string      `json:"sessionID"`
	Message   MessageInfo `json:"message"`
}

// TimeRange is unix-millis start/end; End is 0 while running.
type TimeRange struct {
	Start int64 `json:"start"`
	End   int64 `json:"end"`
}

// ToolState is the live state of a tool call.
type ToolState struct {
	Status   string          `json:"status"` // pending|running|completed|error
	Input    json.RawMessage `json:"input,omitempty"`
	Title    string          `json:"title,omitempty"`
	Output   string          `json:"output,omitempty"`
	Error    string          `json:"error,omitempty"`
	Metadata json.RawMessage `json:"metadata,omitempty"`
	Time     *TimeRange      `json:"time,omitempty"`
}

// Part is a flat union: which fields are meaningful depends on Type.
// Invariant: ID and Type are always set; adapters must not emit unknown types
// (they translate to a known one or drop the part).
type Part struct {
	ID   string `json:"id"`
	Type string `json:"type"`

	// text, reasoning
	Text string     `json:"text,omitempty"`
	Time *TimeRange `json:"time,omitempty"`

	// tool
	CallID string     `json:"callID,omitempty"`
	Tool   string     `json:"tool,omitempty"`
	State  *ToolState `json:"state,omitempty"`

	// step-finish
	Reason string      `json:"reason,omitempty"`
	Cost   float64     `json:"cost,omitempty"`
	Tokens *TokenUsage `json:"tokens,omitempty"`

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
	Path string `json:"path,omitempty"`
}

// PartUpdated carries a full part replacement. Consumers upsert by
// (sessionID, messageID, part.ID); the part is the source of truth, which is
// what makes missed SSE events self-healing (docs/trd.md §3.3).
type PartUpdated struct {
	ProjectID string `json:"projectID"`
	SessionID string `json:"sessionID"`
	MessageID string `json:"messageID"`
	Part      Part   `json:"part"`
}

// PartDelta is a best-effort incremental fast path; a following PartUpdated is
// authoritative. Consumers must tolerate deltas arriving for parts they have
// not seen yet (create a stub).
type PartDelta struct {
	ProjectID string `json:"projectID"`
	SessionID string `json:"sessionID"`
	MessageID string `json:"messageID"`
	PartID    string `json:"partID"`
	Field     string `json:"field"` // "text" | "reasoning"
	Delta     string `json:"delta"`
}

// SessionUpdated fires for create/update; the UI upserts by Session.ID.
type SessionUpdated struct {
	ProjectID string  `json:"projectID"`
	Session   Session `json:"session"`
}

// SessionRemoved fires when a session is deleted server-side.
type SessionRemoved struct {
	ProjectID string `json:"projectID"`
	SessionID string `json:"sessionID"`
}

// PermissionRequest asks the user to approve an agent action.
// Pattern/Metadata are opaque JSON passthrough from the adapter.
type PermissionRequest struct {
	ID        string          `json:"id"`
	Kind      string          `json:"kind,omitempty"` // bash|edit|webfetch|…
	Title     string          `json:"title,omitempty"`
	Pattern   json.RawMessage `json:"pattern,omitempty"`
	Metadata  json.RawMessage `json:"metadata,omitempty"`
	CallID    string          `json:"callID,omitempty"`
	CreatedAt int64           `json:"createdAt,omitempty"` // unix millis
}

// PermissionRequested is the wire event.
type PermissionRequested struct {
	ProjectID  string            `json:"projectID"`
	SessionID  string            `json:"sessionID"`
	Permission PermissionRequest `json:"permission"`
}

// PermissionResolved confirms a reply was accepted server-side.
type PermissionResolved struct {
	ProjectID    string `json:"projectID"`
	SessionID    string `json:"sessionID"`
	PermissionID string `json:"permissionID"`
	Response     string `json:"response"`
}

// AgentError is a normalized, user-presentable error.
type AgentError struct {
	Name    string `json:"name"`
	Message string `json:"message"`
}

// SessionErrored is fatal/terminal for the current turn (retries surface via
// SessionStatus with StatusRetry instead — docs/trd.md §3.3).
type SessionErrored struct {
	ProjectID string     `json:"projectID"`
	SessionID string     `json:"sessionID,omitempty"`
	Error     AgentError `json:"error"`
}

// PromptRequest is what the UI sends (relay → adapter).
type PromptRequest struct {
	Text  string `json:"text"`
	Agent string `json:"agent,omitempty"`
	// Provider/Model as the agent server expects them, e.g. "anthropic"/"claude-…".
	Provider string `json:"provider,omitempty"`
	Model    string `json:"model,omitempty"`
}

// ModelInfo is one selectable model.
type ModelInfo struct {
	ID       string `json:"id"`
	Name     string `json:"name,omitempty"`
	Provider string `json:"provider"`
}

// AgentInfo is one selectable agent (build/plan/…).
type AgentInfo struct {
	Name        string `json:"name"`
	Description string `json:"description,omitempty"`
	Mode        string `json:"mode,omitempty"` // primary|subagent|all
}

// Meta is the picker data for the composer.
type Meta struct {
	Agents []AgentInfo `json:"agents"`
	Models []ModelInfo `json:"models"`
}
