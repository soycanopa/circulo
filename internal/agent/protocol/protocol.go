// Package protocol is the neutral wire contract between the Go backend and the
// React frontend. It is the ONLY shared vocabulary: agent adapters translate
// their vendor events into these types, the UI consumes nothing else.
//
// JSON encoding is camelCase to mirror frontend/src/lib/agent/protocol.ts.
// Any struct change here must land in the same commit as its TS mirror
// (AGENTS.md: Project map).
package protocol

import (
	"encoding/json"
	"sort"
)

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
	EventFormUpdated        = "form.updated"
	EventFormResolved       = "form.resolved"
	EventSessionError       = "session.error"
	EventTodoUpdated        = "todo.updated"
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
//
// Seq and Epoch are the resume contract. Seq is assigned by the orchestrator as
// it fans events in, monotonically from 1 within one Epoch. Epoch changes on
// every process start, so a Seq that restarts at 1 after a restart can never be
// mistaken for a continuation of the old run — without that, resuming a stream
// across a restart is worse than not resuming at all. Both are omitted when
// zero/empty, which is only true for envelopes built outside the orchestrator
// (tests, direct construction).
type Envelope struct {
	Type    string          `json:"type"`
	Seq     uint64          `json:"seq,omitempty"`
	Epoch   string          `json:"epoch,omitempty"`
	Payload json.RawMessage `json:"payload"`
}

// Cursor is where a subscriber left off. An empty Cursor means "no position
// held yet" (first connect, or a page reload): the subscriber wants the whole
// ring and still owes the client a full bootstrap.
type Cursor struct {
	Epoch string
	Seq   uint64
}

// ReplayStatus is the server's answer to a resume request, sent as the first
// SSE control frame. It exists so the client knows whether it must refetch
// (full/stale/gap) or can simply continue (resumed) — today it refetches
// everything on every reconnect because it cannot tell the difference.
type ReplayStatus string

const (
	// ReplayFull: no cursor supplied; the whole ring is replayed.
	ReplayFull ReplayStatus = "full"
	// ReplayResumed: the cursor is valid and contiguous; only the tail after
	// it is replayed and the client is already current.
	ReplayResumed ReplayStatus = "resumed"
	// ReplayStale: the cursor's epoch is not this process's epoch — the app
	// restarted, so the cursor is meaningless.
	ReplayStale ReplayStatus = "stale"
	// ReplayGap: the cursor predates the replay ring, so the events in
	// between no longer exist server-side.
	ReplayGap ReplayStatus = "gap"
)

// ReplayFrame is the SSE control frame carrying a ReplayStatus. It is not an
// Envelope: it describes the stream, not a domain event, and the reducer must
// never see it.
type ReplayFrame struct {
	Status ReplayStatus `json:"status"`
	Epoch  string       `json:"epoch"`
	// Seq is the highest sequence replayed in this connection (0 when the
	// cursor was stale or gapped and nothing was replayed), so the client can
	// seed its own position without waiting for the first event.
	Seq uint64 `json:"seq"`
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

// ContextUsage is the model context-window fill of the live session.
// Window > 0 lets the UI render a percentage; Window == 0 means the backend
// didn't expose one (UsedTokens still renders as a count).
type ContextUsage struct {
	ProjectID string `json:"projectID"`
	SessionID string `json:"sessionID"`
	Used      int64  `json:"used"`
	Window    int64  `json:"window,omitempty"`
}

// EventContextUpdated is the envelope type for ContextUsage.
const EventContextUpdated = "context.updated"

// ContextUpdated fires whenever the session's context fill changes.
type ContextUpdated struct {
	Usage ContextUsage `json:"usage"`
}

// TokenUsage is flat (no nested cache object) so the TS mirror stays simple.
type TokenUsage struct {
	Input      int64 `json:"input"`
	Output     int64 `json:"output"`
	Reasoning  int64 `json:"reasoning"`
	CacheRead  int64 `json:"cacheRead"`
	CacheWrite int64 `json:"cacheWrite"`
}

// MessageInfo is message metadata (identity + accounting). Content lives in Parts.
type MessageInfo struct {
	ID        string      `json:"id"`
	SessionID string      `json:"sessionID"`
	Role      string      `json:"role"`
	Created   int64       `json:"created"`   // unix millis
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

// ProjectVcs is the git state of a project's directory: IsRepo gates the
// branch picker; Branch is the checked-out branch.
type ProjectVcs struct {
	IsRepo        bool   `json:"isRepo"`
	Provider      string `json:"provider,omitempty"`
	Branch        string `json:"branch,omitempty"`
	DefaultBranch string `json:"defaultBranch,omitempty"`
}

// FormOption is one choice of a select-style form field.
type FormOption struct {
	Value       string `json:"value"`
	Label       string `json:"label"`
	Description string `json:"description,omitempty"`
}

// FormField is one question of a form. V0 covers the question tool's shape:
// a string field with options; Custom allows free text instead of a choice.
type FormField struct {
	Key         string       `json:"key"`
	Type        string       `json:"type,omitempty"` // string|multiselect|…
	Title       string       `json:"title,omitempty"`
	Description string       `json:"description,omitempty"`
	Options     []FormOption `json:"options,omitempty"`
	Custom      bool         `json:"custom,omitempty"`
}

// Form is a pending form to answer (opencode v2 question tool). Kind is the
// server metadata.kind ("question"); CallID links the originating tool call.
type Form struct {
	ID     string      `json:"id"`
	Title  string      `json:"title,omitempty"`
	Kind   string      `json:"kind,omitempty"`
	CallID string      `json:"callID,omitempty"`
	Fields []FormField `json:"fields"`
}

// FormUpdated upserts a pending form (reducer keys by Form.ID).
type FormUpdated struct {
	ProjectID string `json:"projectID"`
	SessionID string `json:"sessionID"`
	Form      Form   `json:"form"`
}

// FormResolved closes a form: answered via ReplyForm or cancelled elsewhere.
type FormResolved struct {
	ProjectID string `json:"projectID"`
	SessionID string `json:"sessionID"`
	FormID    string `json:"formID"`
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

// Task is one agent todo item. Status passes through the agent's vocabulary
// (OpenCode: pending | in_progress | completed); the UI renders unknown
// values as pending.
type Task struct {
	ID       string `json:"id,omitempty"`
	Content  string `json:"content"`
	Status   string `json:"status"`
	Priority string `json:"priority,omitempty"`
}

// TodoUpdated replaces the session's task list (OpenCode sends the full list).
type TodoUpdated struct {
	ProjectID string `json:"projectID"`
	SessionID string `json:"sessionID"`
	Tasks     []Task `json:"tasks"`
}

// ModelInfo is one selectable model.
type ModelInfo struct {
	ID       string `json:"id"`
	Name     string `json:"name,omitempty"`
	Provider string `json:"provider"`
	// Reasoning reports capabilities.reasoning from the agent server; only
	// then does the UI offer the effort selector.
	Reasoning bool `json:"reasoning,omitempty"`
	// Variants lists the reasoning-effort options the model accepts
	// (e.g. low/high/max), sorted.
	Variants []string `json:"variants,omitempty"`
}

// AgentInfo is one selectable agent (build/plan/…).
type AgentInfo struct {
	Name        string `json:"name"`
	Description string `json:"description,omitempty"`
	Mode        string `json:"mode,omitempty"` // primary|subagent|all
}

// Access modes for the omp provider. They map 1:1 onto omp's
// tools.approvalMode tiers (always-ask|write|yolo) applied at spawn; there
// is no RPC setter in omp, so changing mode restarts the adapter child.
const (
	// AccessSupervised auto-approves read-only tools only; writes and exec
	// prompt via the omp UI round-trip (extension_ui_request select).
	AccessSupervised = "supervised"
	// AccessEdits auto-approves read and workspace-write tools; exec tools
	// (bash, eval, browser, task) still prompt.
	AccessEdits = "edits"
	// AccessFull auto-approves all tool tiers (omp "yolo"; omp's own default).
	AccessFull = "full"
)

// AccessModeInfo is one selectable access mode for the composer picker.
type AccessModeInfo struct {
	ID          string `json:"id"`
	Title       string `json:"title"`
	Description string `json:"description"`
}

// AccessState is the provider's current access mode. Supported is false for
// providers without an access surface (opencode has no equivalent; the UI
// hides the picker instead of guessing).
type AccessState struct {
	Mode      string `json:"mode,omitempty"`
	Supported bool   `json:"supported"`
}

// Meta is the picker data for the composer.
// Capabilities declares what a backend can actually do.
//
// This exists so the UI never has to infer behavior from the provider's name.
// The distinction matters because the two backends differ in ways that are
// invisible until the user hits them: omp has no permission round-trip, so
// permission cards simply never appear, and its branch picker is a no-op, so
// the chip looks functional and does nothing. Declaring the gap is the
// difference between a feature that is absent and a feature that is broken.
//
// Identity is a separate concern and stays name-based: which product this is,
// its logo, and which "add project" entry to show are legitimately about the
// provider's name.
type Capabilities struct {
	// Permissions is a real approve/deny round-trip (omp: no).
	Permissions bool `json:"permissions"`
	// Forms is a real answer channel for select/confirm/input requests.
	Forms bool `json:"forms"`
	// Steer means a prompt sent while the agent is working is delivered as a
	// steering message rather than starting a second turn (omp: yes).
	Steer bool `json:"steer"`
	// Branch means the branch the session works on can actually be pinned.
	Branch bool `json:"branch"`
	// Agents means the backend exposes a named-agent registry to pick from.
	Agents bool `json:"agents"`
	// Commands means the backend exposes a slash-command surface.
	Commands bool `json:"commands"`
	// AccessMode means the backend exposes an access-mode selector.
	AccessMode bool `json:"accessMode"`
}

// Meta is the composer picker's data.
type Meta struct {
	Agents []AgentInfo `json:"agents"`
	Models []ModelInfo `json:"models"`
	// Server-configured defaults (GET /config/providers → default), so the
	// composer lands on a model the user's plan actually allows.
	DefaultProvider string `json:"defaultProvider,omitempty"`
	DefaultModel    string `json:"defaultModel,omitempty"`
	// Access modes the provider exposes; empty for opencode (no surface).
	AccessModes []AccessModeInfo `json:"accessModes,omitempty"`
	// Current access mode (provider-agnostic id from the Access* constants).
	Access AccessState `json:"access"`
	// Slash commands (built-ins, skills, user commands); nil = provider
	// exposes no command surface.
	Commands []CommandInfo `json:"commands,omitempty"`
	// Capabilities mirrors what this backend can do, so the UI can explain a
	// missing affordance instead of silently omitting it.
	Capabilities Capabilities `json:"capabilities"`
}

// SetAccessRequest is the body of POST /projects/{id}/access.
type SetAccessRequest struct {
	Mode string `json:"mode"`
}

// CommandInfo is one slash command the provider exposes (built-ins, skills,
// user commands). ArgsHint documents the expected arguments, empty when the
// command takes none.
// CommandScope is the neutral origin of a slash command. It replaces the raw
// provider vocabulary that used to cross in CommandInfo.Source: "skill" is
// omp's word, "file" is opencode's, and the UI should not need to know which
// backend it is talking to in order to colour a chip.
type CommandScope string

const (
	// ScopeBuiltin is shipped by the backend itself.
	ScopeBuiltin CommandScope = "builtin"
	// ScopeProject comes from the project the agent is working in.
	ScopeProject CommandScope = "project"
	// ScopeProjectFile is a command file in the project (opencode's "file").
	ScopeProjectFile CommandScope = "projectFile"
	// ScopeUser comes from the user's own command directory.
	ScopeUser CommandScope = "user"
	// ScopeExtension is contributed by a plugin or extension.
	ScopeExtension CommandScope = "extension"
	// ScopeSkill is a skill-provided command.
	ScopeSkill CommandScope = "skill"
)

// displayRank orders the picker. Backends first because they are always
// available; the user's own commands last because those are the ones they
// wrote. This is deliberately NOT the same as ResolveCommands' precedence.
func (c CommandScope) displayRank() int {
	switch c {
	case ScopeBuiltin:
		return 0
	case ScopeProject:
		return 1
	case ScopeUser:
		return 2
	case ScopeSkill:
		return 3
	case ScopeExtension:
		return 4
	case ScopeProjectFile:
		return 5
	}
	return 6
}

// resolutionRank decides which command wins a name collision. A user's own
// project command should shadow a backend builtin, or the builtin would make
// the project's command unreachable — which is the opposite of intent. It is
// intentionally the reverse of displayRank in places: the winner of a
// collision is not the one shown first.
func (c CommandScope) resolutionRank() int {
	switch c {
	case ScopeProject:
		return 5
	case ScopeUser:
		return 4
	case ScopeProjectFile:
		return 3
	case ScopeExtension:
		return 2
	case ScopeSkill:
		return 1
	case ScopeBuiltin:
		return 0
	}
	return -1
}

// NormalizeScope maps a backend's own vocabulary onto the neutral scope. An
// unrecognized value becomes ScopeBuiltin rather than crossing the boundary
// as an unknown string the UI has to guess about.
func NormalizeScope(raw string) CommandScope {
	switch raw {
	case "project":
		return ScopeProject
	case "user":
		return ScopeUser
	case "skill":
		return ScopeSkill
	case "custom", "extension":
		return ScopeExtension
	case "file", "projectFile":
		return ScopeProjectFile
	case "builtin", "waku", "":
		return ScopeBuiltin
	}
	return ScopeBuiltin
}

type CommandInfo struct {
	Name        string `json:"name"`
	Description string `json:"description,omitempty"`
	ArgsHint    string `json:"argsHint,omitempty"`
	// Scope is the neutral origin of the command. The provider's own word
	// does not cross.
	Scope CommandScope `json:"scope,omitempty"`
}

// ResolveCommands collapses same-name commands to the one that wins by
// resolution precedence, then orders the result for display. The two orderings
// are different on purpose: a project command outranks a builtin so it can
// shadow it, while the picker still lists the builtin first. Getting this
// backwards produces either unreachable commands or a picker in which the
// shadowed entry appears to be the winner.
//
// Input order does not matter: the result depends only on names and scopes.
func ResolveCommands(in []CommandInfo) []CommandInfo {
	best := make(map[string]CommandInfo, len(in))
	for _, c := range in {
		if c.Name == "" {
			continue
		}
		prev, seen := best[c.Name]
		if !seen || c.Scope.resolutionRank() > prev.Scope.resolutionRank() {
			best[c.Name] = c
		}
	}
	out := make([]CommandInfo, 0, len(best))
	for _, c := range best {
		out = append(out, c)
	}
	sort.SliceStable(out, func(i, j int) bool {
		ri, rj := out[i].Scope.displayRank(), out[j].Scope.displayRank()
		if ri != rj {
			return ri < rj
		}
		return out[i].Name < out[j].Name
	})
	return out
}

// CommandRequest is the body of POST /projects/{id}/sessions/{sid}/command.
// Name is the command without the leading slash; Text is the raw composer
// input (arguments included) so the provider keeps its own parsing.
type CommandRequest struct {
	Name string `json:"name"`
	Text string `json:"text"`
}

// EventCommandsUpdated is the envelope type for CommandsUpdated.
const EventCommandsUpdated = "commands.updated"

// CommandsUpdated replaces the project's command list (providers refresh it
// when skills/config change); the UI also merges it from Meta.
type CommandsUpdated struct {
	ProjectID string        `json:"projectID"`
	Commands  []CommandInfo `json:"commands"`
}

// PromptRequest is what the UI sends (relay → adapter).
type PromptRequest struct {
	Text  string `json:"text"`
	Agent string `json:"agent,omitempty"`
	// Provider/Model as the agent server expects them, e.g. "anthropic"/"claude-…".
	Provider string `json:"provider,omitempty"`
	Model    string `json:"model,omitempty"`
	// Variant selects a model's reasoning-effort variant (OpenCode prompt
	// body "variant"); empty = the model's default.
	Variant string `json:"variant,omitempty"`
	// Files lists @-mentioned workspace paths (relative to the project root).
	Files []string `json:"files,omitempty"`
}

// FileSearchRequest is the query behind the composer's @-autocomplete.
type FileSearchRequest struct {
	Query string `json:"query"`
	Limit int    `json:"limit,omitempty"`
}

// FileHit is one workspace file match.
type FileHit struct {
	// Path is relative to the project root.
	Path string `json:"path"`
}
