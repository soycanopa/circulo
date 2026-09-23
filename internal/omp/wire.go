// Package omp adapts the omp coding agent (https://omp.sh, fork of Pi) to the
// neutral agent protocol. omp's programmatic surface is `omp --mode rpc`: a
// newline-delimited JSON protocol over the child's stdio (docs/rpc.md in
// can1357/oh-my-pi). There is no HTTP server and no attach mode — one spawned
// process per project, one live session per process.
//
// Dependency rule (AGENTS.md): this package imports only internal/agent,
// internal/agent/protocol, and the stdlib.
package omp

import (
	"encoding/json"
	"fmt"
)

// Wire frame types on the child's stdout. Only `type` is inspected to route;
// payloads decode lazily so unknown forward-compatible frames are cheap to
// drop (AGENTS.md OpenCode traps apply equally here).

type frameType string

const (
	frameReady               frameType = "ready"
	frameChunk               frameType = "rpc_chunk"
	frameResponse            frameType = "response"
	frameAgentStart          frameType = "agent_start"
	frameAgentEnd            frameType = "agent_end"
	frameTurnStart           frameType = "turn_start"
	frameTurnEnd             frameType = "turn_end"
	frameMessageStart        frameType = "message_start"
	frameMessageUpdate       frameType = "message_update"
	frameMessageEnd          frameType = "message_end"
	frameToolExecStart       frameType = "tool_execution_start"
	frameToolExecUpdate      frameType = "tool_execution_update"
	frameToolExecEnd         frameType = "tool_execution_end"
	frameAutoRetryStart      frameType = "auto_retry_start"
	frameAutoRetryEnd        frameType = "auto_retry_end"
	frameExtensionUIRequest  frameType = "extension_ui_request"
	frameModelChanged        frameType = "model_changed"
	frameAutoCompactionStart frameType = "auto_compaction_start"
	frameAutoCompactionEnd   frameType = "auto_compaction_end"
	frameAvailableCommands   frameType = "available_commands_update"
	frameCommandOutput       frameType = "command_output"
)

// envelope is the minimal route key of any stdout frame.
type envelope struct {
	Type frameType `json:"type"`
	ID   string    `json:"id"` // responses correlate by request id
}

// readyFrame advertises protocol versions and transport limits at startup.
type readyFrame struct {
	Type                      string `json:"type"`
	ProtocolVersion           int    `json:"protocolVersion"`
	SupportedProtocolVersions []int  `json:"supportedProtocolVersions"`
	MaxFrameBytes             int64  `json:"maxFrameBytes"`
	MaxReassembledFrameBytes  int64  `json:"maxReassembledFrameBytes"`
}

// chunkFrame is one base64 segment of an oversized stdout frame (protocol v2).
type chunkFrame struct {
	Type       string `json:"type"`
	ChunkID    string `json:"chunkId"`
	Index      int    `json:"index"`
	Count      int    `json:"count"`
	ByteLength int64  `json:"byteLength"`
	Data       string `json:"data"`
}

// responseFrame answers a command. Data is command-specific; failures carry
// Error and an optional machine-readable Code.
type responseFrame struct {
	Type    string          `json:"type"`
	ID      string          `json:"id"`
	Command string          `json:"command"`
	Success bool            `json:"success"`
	Data    json.RawMessage `json:"data"`
	Error   string          `json:"error"`
	Code    string          `json:"code"`
}

// ResponseError converts a failed command response into an error.
func (r *responseFrame) ResponseError() error {
	if r.Success {
		return nil
	}
	if r.Code != "" {
		return fmt.Errorf("omp %s: %s (%s)", r.Command, r.Error, r.Code)
	}
	return fmt.Errorf("omp %s: %s", r.Command, r.Error)
}

// --- session events (AgentSessionEvent subset) -----------------------------

// agentMessage mirrors the pi-ai Message union: role-discriminated, content is
// a string (user/developer, sometimes) or an array of typed blocks.
// toolResult-only fields (toolCallId/toolName/isError/details) are ignored by
// other roles.
type agentMessage struct {
	Role string `json:"role"`
	// Content is raw: string for simple user messages, []block otherwise.
	Content json.RawMessage `json:"content"`
	// Timestamp is unix milliseconds — present on every role.
	Timestamp int64 `json:"timestamp"`

	// toolResult-only fields.
	ToolCallID string          `json:"toolCallId"`
	ToolName   string          `json:"toolName"`
	IsError    bool            `json:"isError"`
	Details    json.RawMessage `json:"details"`

	// Assistant-only metadata.
	Provider     string  `json:"provider"`
	Model        string  `json:"model"`
	Usage        *usage  `json:"usage"`
	StopReason   string  `json:"stopReason"`
	ErrorMessage string  `json:"errorMessage"`
	Duration     float64 `json:"duration"`
	CompletedAt  int64   `json:"completedAt"`
}

// contentBlock is one typed block of message.content.
type contentBlock struct {
	Type string `json:"type"` // text | thinking | image | toolCall | ...
	Text string `json:"text"`
	// ThinkingContent carries its text under "thinking".
	Thinking string `json:"thinking"`
	// ToolCall blocks.
	ToolCallID string          `json:"id"`
	Name       string          `json:"name"`
	Arguments  json.RawMessage `json:"arguments"`
}

// usage mirrors the pi-ai Usage shape (flat counters + nested cost).
type usage struct {
	Input           int64  `json:"input"`
	Output          int64  `json:"output"`
	CacheRead       int64  `json:"cacheRead"`
	CacheWrite      int64  `json:"cacheWrite"`
	TotalTokens     int64  `json:"totalTokens"`
	ReasoningTokens int64  `json:"reasoningTokens"`
	Cost            *costs `json:"cost"`
}

// costs is the nested dollar-cost object (same key names as usage, float
// values).
type costs struct {
	Input      float64 `json:"input"`
	Output     float64 `json:"output"`
	CacheRead  float64 `json:"cacheRead"`
	CacheWrite float64 `json:"cacheWrite"`
	Total      float64 `json:"total"`
}

// totalCost extracts the total dollars spent on the message.
func (u *usage) totalCost() float64 {
	if u == nil || u.Cost == nil {
		return 0
	}
	if u.Cost.Total > 0 {
		return u.Cost.Total
	}
	return u.Cost.Input + u.Cost.Output + u.Cost.CacheRead + u.Cost.CacheWrite
}

// assistantMessageEvent drives streaming deltas (message_update frames).
type assistantMessageEvent struct {
	Type string `json:"type"` // text_delta | thinking_delta | toolcall_start | ...
	// Delta is the raw increment for *_delta events.
	Delta string `json:"delta"`
	// ContentIndex locates the block inside message.content.
	ContentIndex int `json:"contentIndex"`
	// ToolCall is set on toolcall_end with the complete call.
	ToolCall *contentBlock `json:"toolCall"`
}

type messageEventFrame struct {
	Type                  string                 `json:"type"`
	Message               agentMessage           `json:"message"`
	AssistantMessageEvent *assistantMessageEvent `json:"assistantMessageEvent"`
}

type agentEndFrame struct {
	Type       string         `json:"type"`
	Messages   []agentMessage `json:"messages"`
	IsTerminal *bool          `json:"isTerminal"`
}

type toolExecFrame struct {
	Type          string          `json:"type"`
	ToolCallID    string          `json:"toolCallId"`
	ToolName      string          `json:"toolName"`
	Args          json.RawMessage `json:"args"`
	PartialResult json.RawMessage `json:"partialResult"`
	Result        json.RawMessage `json:"result"`
	IsError       bool            `json:"isError"`
}

// toolResultContent decodes AgentToolResult.content blocks (text + images).
type toolResultContent struct {
	Type string `json:"type"`
	Text string `json:"text"`
}

type toolResult struct {
	Content []toolResultContent `json:"content"`
	Details json.RawMessage     `json:"details"`
}

// text extracts the concatenated text blocks (output surfaces).
func (r *toolResult) text() string {
	if r == nil {
		return ""
	}
	out := ""
	for i, b := range r.Content {
		if b.Type != "text" || b.Text == "" {
			continue
		}
		if i > 0 && out != "" {
			out += "\n"
		}
		out += b.Text
	}
	return out
}

type retryFrame struct {
	Type    string `json:"type"`
	Attempt int    `json:"attempt"`
	Message string `json:"message"`
}

// extensionUIRequest is the interactive round-trip (select/confirm/input and
// presentation-only methods we drop).
type extensionUIRequest struct {
	Type          string   `json:"type"`
	ID            string   `json:"id"`
	Method        string   `json:"method"`
	Title         string   `json:"title"`
	Message       string   `json:"message"`
	Options       []string `json:"options"`
	OptionDetails []struct {
		Description string `json:"description"`
	} `json:"optionDetails"`
	Placeholder string `json:"placeholder"`
}

// --- get_state / get_available_models payloads ------------------------------

type sessionState struct {
	Model *modelRef `json:"model"`
	// ThinkingLevel: off|minimal|low|medium|high|xhigh|max.
	ThinkingLevel string `json:"thinkingLevel"`
	IsStreaming   bool   `json:"isStreaming"`
	IsCompacting  bool   `json:"isCompacting"`
	SessionFile   string `json:"sessionFile"`
	SessionID     string `json:"sessionId"`
	SessionName   string `json:"sessionName"`
	MessageCount  int    `json:"messageCount"`
	// ContextUsage is the live context-window fill omp computes per turn.
	ContextUsage *struct {
		Tokens        int64   `json:"tokens"`
		ContextWindow int64   `json:"contextWindow"`
		Percent       float64 `json:"percent"`
	} `json:"contextUsage"`
}

type modelRef struct {
	Provider string `json:"provider"`
	ID       string `json:"id"`
}

type modelsPayload struct {
	Models []ompModel `json:"models"`
}

// ompModel keeps only the fields the composer needs; the catalog entries are
// much larger (contextWindow, cost, tokenizer, ...).
type ompModel struct {
	ID        string `json:"id"`
	Name      string `json:"name"`
	Provider  string `json:"provider"`
	Reasoning bool   `json:"reasoning"`
	Thinking  *struct {
		Efforts []string `json:"efforts"`
	} `json:"thinking"`
}

// --- outbound commands ------------------------------------------------------

// command builds one stdin line. ID is assigned by the client when "".
type command struct {
	id      string
	payload map[string]any
}

func newCommand(cmd string) command {
	return command{payload: map[string]any{"type": cmd}}
}

func (c command) with(key string, value any) command {
	c.payload[key] = value
	return c
}

func (c command) marshal(id string) ([]byte, error) {
	if id != "" {
		c.payload["id"] = id
	}
	return json.Marshal(c.payload)
}
