package omp

import (
	"encoding/json"
	"fmt"
	"time"

	"circulogo/internal/agent/protocol"
)

// translator converts omp session-event frames into neutral envelopes. It is
// stateful in exactly one way: tool_execution frames do not name their owning
// message, so the translator remembers the latest assistant message id (tool
// calls execute after their assistant message ends) and stamps it on tool
// part updates. The reducer keys parts by (sessionID, messageID, partID);
// without this, tool parts would land in a phantom message.
//
// Unknown frame types return nil envelopes (forward compatibility: unknown
// vendor events must never error — AGENTS.md).
type translator struct {
	projectID string
	ids       *messageIDs
	// lastAssistantID is the messageID of the most recent assistant message.
	lastAssistantID string
	// toolInputs remembers each call's decoded input: part updates are full
	// replacements, so later tool frames (execution end, toolResult end) must
	// re-emit the input captured at toolcall time.
	toolInputs map[string]json.RawMessage
}

func newTranslator(projectID string) *translator {
	return &translator{
		projectID:  projectID,
		ids:        newMessageIDs(),
		toolInputs: map[string]json.RawMessage{},
	}
}

// reset re-targets the translator after a session switch: identities are
// per-session ordinals.
func (t *translator) reset() {
	t.lastAssistantID = ""
	t.ids = newMessageIDs()
	t.toolInputs = map[string]json.RawMessage{}
}

func (t *translator) translate(raw json.RawMessage, typ frameType, sessionID string) ([]protocol.Envelope, error) {
	switch typ {
	case frameAgentStart:
		t.lastAssistantID = ""
		return t.status(sessionID, protocol.StatusBusy, nil)
	case frameAgentEnd:
		var f agentEndFrame
		if err := json.Unmarshal(raw, &f); err != nil {
			return nil, badFrame(typ, err)
		}
		// isTerminal === false: more work is scheduled; the run is not over.
		if f.IsTerminal != nil && !*f.IsTerminal {
			return nil, nil
		}
		t.lastAssistantID = ""
		return t.status(sessionID, protocol.StatusIdle, nil)

	case frameAutoRetryStart:
		var f retryFrame
		_ = json.Unmarshal(raw, &f) // optional fields only
		return t.status(sessionID, protocol.StatusRetry, &protocol.RetryInfo{
			Attempt: f.Attempt, Message: f.Message,
		})
	case frameAutoRetryEnd:
		return t.status(sessionID, protocol.StatusBusy, nil)

	case frameMessageStart:
		return t.messageStart(raw, sessionID)
	case frameMessageUpdate:
		return t.messageUpdate(raw, sessionID)
	case frameMessageEnd:
		return t.messageEnd(raw, sessionID)

	case frameToolExecStart, frameToolExecUpdate, frameToolExecEnd:
		return t.toolExec(raw, typ, sessionID)

	case frameExtensionUIRequest:
		return t.extensionUI(raw, sessionID)

	case frameAvailableCommands:
		return t.commands(raw)

	case frameCommandOutput:
		// Slash-command output: omp emits it as a free frame (no request id)
		// right before the prompt response — text the command printed, not a
		// model turn. Same shape as the user echo: message.updated for the
		// shell message, part.updated for its text.
		var f struct {
			Text string `json:"text"`
		}
		if err := json.Unmarshal(raw, &f); err != nil {
			return nil, badFrame(frameCommandOutput, err)
		}
		if f.Text == "" {
			return nil, nil
		}
		id := t.ids.next("system")
		return envs(
			protocol.EventMessageUpdated, protocol.MessageUpdated{
				ProjectID: t.projectID, SessionID: sessionID,
				Message: protocol.MessageInfo{
					ID: id, SessionID: sessionID, Role: protocol.RoleAssistant,
					Agent: "system", Created: time.Now().UnixMilli(),
				},
			},
			protocol.EventPartUpdated, protocol.PartUpdated{
				ProjectID: t.projectID, SessionID: sessionID, MessageID: id,
				Part: protocol.Part{ID: id + ":text", Type: protocol.PartText, Text: f.Text},
			},
		)

	default:
		// turn_start/turn_end, model_changed, compaction, subagent frames,
		// widget/title chatter, unknown future events: no neutral surface.
		return nil, nil
	}
}

// commands maps omp's available_commands_update (full list) onto the neutral
// commands.updated envelope.
func (t *translator) commands(raw json.RawMessage) ([]protocol.Envelope, error) {
	var f struct {
		Commands []ompCommand `json:"commands"`
	}
	if err := json.Unmarshal(raw, &f); err != nil {
		return nil, badFrame(frameAvailableCommands, err)
	}
	return envs(protocol.EventCommandsUpdated, protocol.CommandsUpdated{
		ProjectID: t.projectID,
		Commands:  ompCommandsToNeutral(f.Commands),
	})
}

// ompCommand is one entry of omp's command list.
type ompCommand struct {
	Name        string `json:"name"`
	Description string `json:"description,omitempty"`
	Input       *struct {
		Hint string `json:"hint,omitempty"`
	} `json:"input"`
	Source string `json:"source,omitempty"`
}

func ompCommandsToNeutral(in []ompCommand) []protocol.CommandInfo {
	out := make([]protocol.CommandInfo, 0, len(in))
	for _, c := range in {
		ci := protocol.CommandInfo{
			Name:        c.Name,
			Description: c.Description,
			Source:      c.Source,
		}
		if c.Input != nil {
			ci.ArgsHint = c.Input.Hint
		}
		out = append(out, ci)
	}
	return out
}

func badFrame(typ frameType, err error) error {
	return fmt.Errorf("omp: bad %s frame: %w", typ, err)
}

func (t *translator) status(sessionID, status string, retry *protocol.RetryInfo) ([]protocol.Envelope, error) {
	return envs(protocol.EventSessionStatus, protocol.SessionStatus{
		ProjectID: t.projectID,
		SessionID: sessionID,
		Status:    status,
		Retry:     retry,
	})
}

// messageIDs assigns stable message identity. omp frames carry no message id,
// so identity is the message's chronological ordinal within its role ("a1",
// "u1", ...). Both identity sources walk the same sequence — the live event
// stream and get_messages replay an append-only session in order — so ids
// agree across streaming, hydration, and adapter restarts. That is the
// self-healing invariant: hydration and the live stream must produce the same
// part ids (docs/trd.md §3.3). Content hashing is deliberately avoided: the
// streaming `message` object grows as deltas land, so a content hash would
// change identity mid-stream and orphan already-emitted parts.
type messageIDs struct{ counts map[string]int }

func newMessageIDs() *messageIDs { return &messageIDs{counts: map[string]int{}} }

// next allocates the id for the next message of this role.
func (m *messageIDs) next(role string) string {
	m.counts[role]++
	switch role {
	case "user":
		return fmt.Sprintf("u%d", m.counts[role])
	case "assistant":
		return fmt.Sprintf("a%d", m.counts[role])
	default:
		return fmt.Sprintf("%s%d", role, m.counts[role])
	}
}

// messageBlocks decodes the content array of a message (assistant/toolResult
// and rich user messages). nil for string content or undecodable payloads.
func messageBlocks(m *agentMessage) []contentBlock {
	if len(m.Content) == 0 || m.Content[0] != '[' {
		return nil
	}
	var blocks []contentBlock
	if err := json.Unmarshal(m.Content, &blocks); err != nil {
		return nil
	}
	return blocks
}

// userText extracts a user message's text (string content or text blocks).
func userText(m *agentMessage) string {
	if len(m.Content) > 0 && m.Content[0] == '"' {
		var s string
		if err := json.Unmarshal(m.Content, &s); err == nil {
			return s
		}
	}
	var texts []string
	for _, b := range messageBlocks(m) {
		if b.Type == "text" && b.Text != "" {
			texts = append(texts, b.Text)
		}
	}
	return joinTexts(texts)
}

// blockAt returns the content block at index i, or an empty block.
func blockAt(m *agentMessage, i int) contentBlock {
	blocks := messageBlocks(m)
	if i < 0 || i >= len(blocks) {
		return contentBlock{}
	}
	return blocks[i]
}

func joinTexts(texts []string) string {
	out := ""
	for _, s := range texts {
		if out != "" {
			out += "\n"
		}
		out += s
	}
	return out
}

// --- message lifecycle ------------------------------------------------------

func (t *translator) messageStart(raw json.RawMessage, sessionID string) ([]protocol.Envelope, error) {
	var f messageEventFrame
	if err := json.Unmarshal(raw, &f); err != nil {
		return nil, badFrame(frameMessageStart, err)
	}
	m := f.Message
	switch m.Role {
	case "user":
		// User echo: replaces the optimistic client_ bubble in the UI.
		id := t.ids.next("user")
		return envs(protocol.EventMessageUpdated, protocol.MessageUpdated{
			ProjectID: t.projectID, SessionID: sessionID,
			Message: protocol.MessageInfo{
				ID: id, SessionID: sessionID, Role: protocol.RoleUser, Created: m.Timestamp,
			},
		},
			protocol.EventPartUpdated, protocol.PartUpdated{
				ProjectID: t.projectID, SessionID: sessionID, MessageID: id,
				Part: protocol.Part{ID: id + ":text", Type: protocol.PartText, Text: userText(&m)},
			})
	case "assistant":
		t.lastAssistantID = t.ids.next("assistant")
		return envs(protocol.EventMessageUpdated, protocol.MessageUpdated{
			ProjectID: t.projectID, SessionID: sessionID,
			Message: protocol.MessageInfo{
				ID: t.lastAssistantID, SessionID: sessionID, Role: protocol.RoleAssistant,
				Created: m.Timestamp, Provider: m.Provider, Model: m.Model,
			},
		})
	default:
		// toolResult start: the tool part is driven by tool_execution events.
		return nil, nil
	}
}

func (t *translator) messageUpdate(raw json.RawMessage, sessionID string) ([]protocol.Envelope, error) {
	var f messageEventFrame
	if err := json.Unmarshal(raw, &f); err != nil {
		return nil, badFrame(frameMessageUpdate, err)
	}
	delta := f.AssistantMessageEvent
	if delta == nil {
		return nil, nil
	}
	m := f.Message
	if m.Role != "assistant" {
		return nil, nil
	}
	// Updates stream the message announced by message_start; a missing start
	// (defensive) allocates the id here instead.
	if t.lastAssistantID == "" {
		t.lastAssistantID = t.ids.next("assistant")
	}
	id := t.lastAssistantID

	switch delta.Type {
	case "text_delta", "thinking_delta":
		partID, partType := id+":text", protocol.PartText
		field, blockText := "text", blockAt(&m, delta.ContentIndex).Text
		if delta.Type == "thinking_delta" {
			partID, partType = id+":reasoning", protocol.PartReasoning
			field, blockText = "reasoning", blockAt(&m, delta.ContentIndex).Thinking
		}
		return envs(protocol.EventPartDelta, protocol.PartDelta{
			ProjectID: t.projectID, SessionID: sessionID, MessageID: id,
			PartID: partID, Field: field, Delta: delta.Delta,
		},
			protocol.EventPartUpdated, protocol.PartUpdated{
				ProjectID: t.projectID, SessionID: sessionID, MessageID: id,
				Part: protocol.Part{ID: partID, Type: partType, Text: blockText},
			})

	case "text_end", "thinking_end":
		partID, partType := id+":text", protocol.PartText
		b := blockAt(&m, delta.ContentIndex)
		text := b.Text
		if delta.Type == "thinking_end" {
			partID, partType = id+":reasoning", protocol.PartReasoning
			text = b.Thinking
		}
		return envs(protocol.EventPartUpdated, protocol.PartUpdated{
			ProjectID: t.projectID, SessionID: sessionID, MessageID: id,
			Part: protocol.Part{ID: partID, Type: partType, Text: text},
		})

	case "toolcall_end":
		tc := delta.ToolCall
		if tc == nil {
			b := blockAt(&m, delta.ContentIndex)
			if b.Type == "toolCall" {
				tc = &contentBlock{Type: "toolCall", ToolCallID: b.ToolCallID, Name: b.Name, Arguments: b.Arguments}
			}
		}
		if tc == nil || tc.ToolCallID == "" {
			return nil, nil
		}
		t.toolInputs[tc.ToolCallID] = tc.Arguments
		return envs(protocol.EventPartUpdated, protocol.PartUpdated{
			ProjectID: t.projectID, SessionID: sessionID, MessageID: id,
			Part: protocol.Part{
				ID: tc.ToolCallID, Type: protocol.PartTool, CallID: tc.ToolCallID, Tool: tc.Name,
				State: &protocol.ToolState{Status: protocol.ToolPending, Input: tc.Arguments},
			},
		})

	default:
		// start/toolcall_start/image_end/toolcall_delta: nothing user-visible
		// yet (the pending tool part appears at toolcall_end or execution).
		return nil, nil
	}
}

func (t *translator) messageEnd(raw json.RawMessage, sessionID string) ([]protocol.Envelope, error) {
	var f messageEventFrame
	if err := json.Unmarshal(raw, &f); err != nil {
		return nil, badFrame(frameMessageEnd, err)
	}
	m := f.Message
	switch m.Role {
	case "assistant":
		// The message just announced (or updated) as lastAssistantID ends.
		if t.lastAssistantID == "" {
			t.lastAssistantID = t.ids.next("assistant")
		}
		id := t.lastAssistantID

		var parts []protocol.Part
		for i, b := range messageBlocks(&m) {
			switch b.Type {
			case "text":
				parts = append(parts, protocol.Part{ID: id + ":text", Type: protocol.PartText, Text: b.Text})
			case "thinking":
				parts = append(parts, protocol.Part{ID: id + ":reasoning", Type: protocol.PartReasoning, Text: b.Thinking})
			}
			_ = i
		}
		out := make([]protocol.Envelope, 0, len(parts)+3)
		for _, p := range parts {
			envsOne, err := envs(protocol.EventPartUpdated, protocol.PartUpdated{
				ProjectID: t.projectID, SessionID: sessionID, MessageID: id, Part: p,
			})
			if err != nil {
				return nil, err
			}
			out = append(out, envsOne...)
		}

		info := protocol.MessageInfo{
			ID: id, SessionID: sessionID, Role: protocol.RoleAssistant,
			Created: m.Timestamp, Provider: m.Provider, Model: m.Model,
			Cost:      m.Usage.totalCost(),
			Finish:    m.StopReason,
			ErrorName: stopErrorName(m.StopReason),
			ErrorMsg:  m.ErrorMessage,
		}
		if m.CompletedAt > 0 {
			info.Completed = m.CompletedAt
		} else if m.Duration > 0 {
			info.Completed = m.Timestamp + int64(m.Duration)
		}
		if m.Usage != nil {
			info.Tokens = &protocol.TokenUsage{
				Input:      m.Usage.Input,
				Output:     m.Usage.Output,
				Reasoning:  m.Usage.ReasoningTokens,
				CacheRead:  m.Usage.CacheRead,
				CacheWrite: m.Usage.CacheWrite,
			}
		}
		msgEnvs, err := envs(protocol.EventMessageUpdated, protocol.MessageUpdated{
			ProjectID: t.projectID, SessionID: sessionID, Message: info,
		})
		if err != nil {
			return nil, err
		}
		out = append(out, msgEnvs...)

		// step-finish part carries the accounting, mirroring the opencode
		// adapter's <msgID>:finish part.
		finEnvs, err := envs(protocol.EventPartUpdated, protocol.PartUpdated{
			ProjectID: t.projectID, SessionID: sessionID, MessageID: id,
			Part: protocol.Part{
				ID: id + ":finish", Type: protocol.PartStepFinish,
				Reason: m.StopReason, Cost: info.Cost, Tokens: info.Tokens,
			},
		})
		if err != nil {
			return nil, err
		}
		out = append(out, finEnvs...)
		return out, nil

	case "toolResult":
		// Durable tool result: finalize the tool part keyed by toolCallId.
		// Part updates are full replacements, so the input captured at
		// toolcall time rides along.
		status, errText := protocol.ToolCompleted, ""
		if m.IsError {
			status, errText = protocol.ToolError, userText(&m)
		}
		return envs(protocol.EventPartUpdated, protocol.PartUpdated{
			ProjectID: t.projectID, SessionID: sessionID, MessageID: t.lastAssistantID,
			Part: protocol.Part{
				ID: m.ToolCallID, Type: protocol.PartTool, CallID: m.ToolCallID, Tool: m.ToolName,
				State: &protocol.ToolState{
					Status: status, Input: t.toolInputs[m.ToolCallID],
					Output: userText(&m), Error: errText, Metadata: m.Details,
					Time: &protocol.TimeRange{Start: m.Timestamp, End: m.Timestamp},
				},
			},
		})

	default:
		return nil, nil
	}
}

// stopErrorName classifies non-success stop reasons for MessageInfo.
func stopErrorName(reason string) string {
	switch reason {
	case "error", "aborted":
		return reason
	default:
		return ""
	}
}

// --- tool execution ---------------------------------------------------------

func (t *translator) toolExec(raw json.RawMessage, typ frameType, sessionID string) ([]protocol.Envelope, error) {
	var f toolExecFrame
	if err := json.Unmarshal(raw, &f); err != nil {
		return nil, badFrame(typ, err)
	}
	// Only tool_execution_start carries args; later frames must re-emit the
	// remembered input (full-replacement updates).
	input := f.Args
	if len(input) > 0 {
		t.toolInputs[f.ToolCallID] = input
	} else {
		input = t.toolInputs[f.ToolCallID]
	}
	state := &protocol.ToolState{Status: protocol.ToolRunning, Input: input}
	switch typ {
	case frameToolExecStart:
	case frameToolExecUpdate:
		var pr toolResult
		if json.Unmarshal(f.PartialResult, &pr) == nil {
			state.Output = pr.text()
			state.Metadata = pr.Details
		}
	case frameToolExecEnd:
		var res toolResult
		if json.Unmarshal(f.Result, &res) == nil {
			state.Output = res.text()
			state.Metadata = res.Details
		}
		if f.IsError {
			state.Status = protocol.ToolError
			state.Error = state.Output
		} else {
			state.Status = protocol.ToolCompleted
		}
	}
	return envs(protocol.EventPartUpdated, protocol.PartUpdated{
		ProjectID: t.projectID, SessionID: sessionID, MessageID: t.lastAssistantID,
		Part: protocol.Part{
			ID: f.ToolCallID, Type: protocol.PartTool, CallID: f.ToolCallID, Tool: f.ToolName,
			State: state,
		},
	})
}

// --- extension UI -----------------------------------------------------------

func (t *translator) extensionUI(raw json.RawMessage, sessionID string) ([]protocol.Envelope, error) {
	var f extensionUIRequest
	if err := json.Unmarshal(raw, &f); err != nil {
		return nil, badFrame(frameExtensionUIRequest, err)
	}
	var fields []protocol.FormField
	switch f.Method {
	case "select":
		field := protocol.FormField{Key: "value", Type: "string", Title: f.Title}
		for i, o := range f.Options {
			opt := protocol.FormOption{Value: o, Label: o}
			if i < len(f.OptionDetails) && f.OptionDetails[i].Description != "" {
				opt.Description = f.OptionDetails[i].Description
			}
			field.Options = append(field.Options, opt)
		}
		fields = []protocol.FormField{field}
	case "confirm":
		fields = []protocol.FormField{{
			Key: "value", Type: "string", Title: f.Title,
			Options: []protocol.FormOption{
				{Value: "true", Label: "Confirm"},
				{Value: "false", Label: "Cancel"},
			},
		}}
	case "input":
		fields = []protocol.FormField{{
			Key: "value", Type: "string", Title: f.Title,
			Description: f.Placeholder, Custom: true,
		}}
	default:
		// notify/setWidget/setTitle/editor/open_url/set_editor_text: no
		// neutral v0 surface.
		return nil, nil
	}
	return envs(protocol.EventFormUpdated, protocol.FormUpdated{
		ProjectID: t.projectID, SessionID: sessionID,
		Form: protocol.Form{ID: f.ID, Title: f.Title, Kind: "omp:" + f.Method, Fields: fields},
	})
}

// --- envelope helpers -------------------------------------------------------

// envs builds one envelope per (event, payload) pair.
func envs(pairs ...any) ([]protocol.Envelope, error) {
	out := make([]protocol.Envelope, 0, len(pairs)/2)
	for i := 0; i < len(pairs); i += 2 {
		typ, ok := pairs[i].(string)
		if !ok {
			panic("omp: envs: odd argument list")
		}
		env, err := protocol.NewEnvelope(typ, pairs[i+1])
		if err != nil {
			return nil, err
		}
		out = append(out, env)
	}
	return out, nil
}
