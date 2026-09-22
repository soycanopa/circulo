package omp

import (
	"encoding/json"
	"fmt"

	"circulogo/internal/agent"
	"circulogo/internal/agent/protocol"
)

// messagesPayload is the get_messages response data.
type messagesPayload struct {
	Messages []agentMessage `json:"messages"`
}

// hydrateMessages converts omp messages (chronological, oldest first) into
// neutral hydrated messages. Part ids match the live translator: hash-derived
// <msgID> with :text/:reasoning/:finish suffixes, provider toolCall ids for
// tool parts. That equality is what makes the reducer's history merge land on
// the same cells the live stream already filled (docs/trd.md §3.3).
//
// omp models tool results as separate toolResult messages; the neutral
// contract folds them into the assistant message's tool part (state.output),
// mirroring how the opencode server represents tool calls.
func hydrateMessages(sessionID string, msgs []agentMessage, limit int) ([]agent.HydratedMessage, error) {
	out := make([]agent.HydratedMessage, 0, len(msgs))
	ids := newMessageIDs()
	// toolCallID -> location of the open tool part (message index, part index).
	type openTool struct{ msg, part int }
	openTools := map[string]openTool{}

	for i := range msgs {
		m := &msgs[i]
		switch m.Role {
		case "user":
			id := ids.next("user")
			out = append(out, agent.HydratedMessage{
				Info: protocol.MessageInfo{
					ID: id, SessionID: sessionID,
					Role: protocol.RoleUser, Created: m.Timestamp,
				},
				Parts: []protocol.Part{{
					ID: id + ":text", Type: protocol.PartText, Text: userText(m),
				}},
			})

		case "assistant":
			id := ids.next("assistant")
			parts := make([]protocol.Part, 0, len(messageBlocks(m))+1)
			for _, b := range messageBlocks(m) {
				switch b.Type {
				case "text":
					parts = append(parts, protocol.Part{ID: id + ":text", Type: protocol.PartText, Text: b.Text})
				case "thinking":
					parts = append(parts, protocol.Part{ID: id + ":reasoning", Type: protocol.PartReasoning, Text: b.Thinking})
				case "toolCall":
					openTools[b.ToolCallID] = openTool{msg: len(out), part: len(parts)}
					parts = append(parts, protocol.Part{
						ID: b.ToolCallID, Type: protocol.PartTool, CallID: b.ToolCallID, Tool: b.Name,
						State: &protocol.ToolState{Status: protocol.ToolPending, Input: b.Arguments},
					})
				}
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
			if info.Finish != "" || info.Cost > 0 || info.Tokens != nil {
				parts = append(parts, protocol.Part{
					ID: id + ":finish", Type: protocol.PartStepFinish,
					Reason: info.Finish, Cost: info.Cost, Tokens: info.Tokens,
				})
			}
			out = append(out, agent.HydratedMessage{Info: info, Parts: parts})

		case "toolResult":
			// Finalize the matching tool part on the assistant message that
			// issued the call.
			loc, ok := openTools[m.ToolCallID]
			if !ok || loc.msg >= len(out) {
				continue
			}
			target := &out[loc.msg]
			if loc.part >= len(target.Parts) {
				continue
			}
			part := target.Parts[loc.part]
			if part.State == nil {
				continue
			}
			status, errText := protocol.ToolCompleted, ""
			if m.IsError {
				status, errText = protocol.ToolError, userText(m)
			}
			part.State.Status = status
			part.State.Output = userText(m)
			part.State.Error = errText
			part.State.Metadata = m.Details
			part.State.Time = &protocol.TimeRange{Start: target.Info.Created, End: m.Timestamp}
			target.Parts[loc.part] = part
			delete(openTools, m.ToolCallID)

		default:
			// developer/system messages: no renderer — dropped in v0.
		}
	}

	if limit > 0 && len(out) > limit {
		out = out[len(out)-limit:]
	}
	if len(out) == 0 {
		return nil, fmt.Errorf("omp: session %s has no renderable messages", sessionID)
	}
	return out, nil
}

// parseMessagesResponse decodes a get_messages response body.
func parseMessagesResponse(data json.RawMessage) ([]agentMessage, error) {
	var p messagesPayload
	if err := json.Unmarshal(data, &p); err != nil {
		return nil, fmt.Errorf("omp: bad get_messages data: %w", err)
	}
	return p.Messages, nil
}
