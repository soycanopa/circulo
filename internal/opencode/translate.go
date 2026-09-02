package opencode

import (
	"encoding/json"
	"fmt"
	"time"

	"circulogo/internal/agent/protocol"
)

// Translate converts one OpenCode event into zero or more neutral protocol
// envelopes. Zero results are the normal path for the ignore-list (docs/
// trd.md §3.3): the stream carries many event types circuloGo does not render,
// and unknown future types must degrade to "ignored", never to errors.
func Translate(projectID string, env Envelope) ([]protocol.Envelope, error) {
	switch env.Type {
	case "server.connected":
		return emit(protocol.EventAdapterStatus, protocol.AdapterStatus{
			ProjectID: projectID,
			State:     protocol.AdapterRunning,
		})

	case "session.created", "session.updated":
		var p EventPropertiesSession
		if err := json.Unmarshal(env.Properties, &p); err != nil {
			return nil, fmt.Errorf("opencode: %s: %w", env.Type, err)
		}
		return emit(protocol.EventSessionUpdated, protocol.SessionUpdated{
			ProjectID: projectID,
			Session:   sessionToNeutral(p.Info),
		})

	case "session.deleted":
		var p EventPropertiesSession
		if err := json.Unmarshal(env.Properties, &p); err != nil {
			return nil, fmt.Errorf("opencode: session.deleted: %w", err)
		}
		return emit(protocol.EventSessionRemoved, protocol.SessionRemoved{
			ProjectID: projectID,
			SessionID: p.Info.ID,
		})

	case "session.status":
		var p EventPropertiesSessionStatus
		if err := json.Unmarshal(env.Properties, &p); err != nil {
			return nil, fmt.Errorf("opencode: session.status: %w", err)
		}
		st := protocol.SessionStatus{
			ProjectID: projectID,
			SessionID: p.SessionID,
		}
		switch p.Status.Type {
		case "busy":
			st.Status = protocol.StatusBusy
		case "retry":
			st.Status = protocol.StatusRetry
			st.Retry = &protocol.RetryInfo{
				Attempt: p.Status.Attempt,
				Message: p.Status.Message,
				NextAt:  p.Status.Next,
			}
		case "idle":
			st.Status = protocol.StatusIdle
		default:
			// Unknown status: map to busy (conservative — the session is not
			// idle) rather than dropping the event.
			st.Status = protocol.StatusBusy
		}
		return emit(protocol.EventSessionStatus, st)

	case "session.idle":
		// Turn-end signal; session.status{idle} carries the same information
		// for our UI, so idle folds into nothing extra (docs/trd.md §5.1).
		return nil, nil

	case "message.updated":
		var p EventPropertiesMessage
		if err := json.Unmarshal(env.Properties, &p); err != nil {
			return nil, fmt.Errorf("opencode: message.updated: %w", err)
		}
		return emit(protocol.EventMessageUpdated, protocol.MessageUpdated{
			ProjectID: projectID,
			SessionID: p.Info.SessionID,
			Message:   messageToNeutral(p.Info),
		})

	case "message.removed", "message.part.removed":
		// Removal only happens on revert flows, which are v1 (docs/implement.md
		// backlog). A stale part is cosmetically wrong but never blocks.
		return nil, nil

	case "message.part.updated":
		var p EventPropertiesPartUpdated
		if err := json.Unmarshal(env.Properties, &p); err != nil {
			return nil, fmt.Errorf("opencode: message.part.updated: %w", err)
		}
		part, ok := partToNeutral(p.Part)
		if !ok {
			return nil, nil // dropped type (snapshot/retry/compaction) in v0
		}
		out := []protocol.Envelope{}
		env2, err := protocol.NewEnvelope(protocol.EventPartUpdated, protocol.PartUpdated{
			ProjectID: projectID,
			SessionID: p.Part.SessionID,
			MessageID: p.Part.MessageID,
			Part:      part,
		})
		if err != nil {
			return nil, err
		}
		out = append(out, env2)
		// Forward the optional delta as a separate best-effort hint so the UI
		// can paint before the (potentially large) full part arrives.
		if p.Delta != nil && *p.Delta != "" {
			d, err := protocol.NewEnvelope(protocol.EventPartDelta, protocol.PartDelta{
				ProjectID: projectID,
				SessionID: p.Part.SessionID,
				MessageID: p.Part.MessageID,
				PartID:    p.Part.ID,
				Field:     partFieldType(p.Part.Type),
				Delta:     *p.Delta,
			})
			if err != nil {
				return nil, err
			}
			out = append(out, d)
		}
		return out, nil

	case "message.part.delta":
		var p EventPropertiesPartDelta
		if err := json.Unmarshal(env.Properties, &p); err != nil {
			return nil, fmt.Errorf("opencode: message.part.delta: %w", err)
		}
		return emit(protocol.EventPartDelta, protocol.PartDelta{
			ProjectID: projectID,
			SessionID: p.SessionID,
			MessageID: p.MessageID,
			PartID:    p.PartID,
			Field:     p.Field,
			Delta:     p.Delta,
		})

	case "todo.updated":
		var p EventPropertiesTodoUpdated
		if err := json.Unmarshal(env.Properties, &p); err != nil {
			return nil, fmt.Errorf("opencode: todo.updated: %w", err)
		}
		tasks := make([]protocol.Task, 0, len(p.Todos))
		for _, t := range p.Todos {
			tasks = append(tasks, protocol.Task{
				ID:       t.ID,
				Content:  t.Content,
				Status:   t.Status,
				Priority: t.Priority,
			})
		}
		return emit(protocol.EventTodoUpdated, protocol.TodoUpdated{
			ProjectID: projectID,
			SessionID: p.SessionID,
			Tasks:     tasks,
		})

	case "permission.asked":
		var p EventPropertiesPermissionAsked
		if err := json.Unmarshal(env.Properties, &p); err != nil {
			return nil, fmt.Errorf("opencode: permission.asked: %w", err)
		}
		callID := ""
		if p.Data.Tool != nil {
			callID = p.Data.Tool.CallID
		}
		return emit(protocol.EventPermissionRequest, protocol.PermissionRequested{
			ProjectID: projectID,
			SessionID: p.Data.SessionID,
			Permission: protocol.PermissionRequest{
				ID:        p.Data.ID,
				Kind:      p.Data.Permission,
				Pattern:   patternsToRaw(p.Data.Patterns),
				Metadata:  p.Data.Metadata,
				CallID:    callID,
				CreatedAt: time.Now().UnixMilli(),
			},
		})

	case "permission.replied":
		var p EventPropertiesPermissionReplied
		if err := json.Unmarshal(env.Properties, &p); err != nil {
			return nil, fmt.Errorf("opencode: permission.replied: %w", err)
		}
		return emit(protocol.EventPermissionResolved, protocol.PermissionResolved{
			ProjectID:    projectID,
			SessionID:    p.Data.SessionID,
			PermissionID: p.Data.RequestID,
			Response:     p.Data.Reply,
		})

	case "session.error":
		var p EventPropertiesSessionError
		if err := json.Unmarshal(env.Properties, &p); err != nil {
			return nil, fmt.Errorf("opencode: session.error: %w", err)
		}
		if p.Error == nil {
			return nil, nil
		}
		return emit(protocol.EventSessionError, protocol.SessionErrored{
			ProjectID: projectID,
			SessionID: p.SessionID,
			Error: protocol.AgentError{
				Name:    p.Error.Name,
				Message: p.Error.Data.Message,
			},
		})

	default:
		// Ignore-list + forward compatibility. Anything not handled above is
		// dropped by design (docs/trd.md §3.3); log-free to avoid hot-loop spam.
		return nil, nil
	}
}

func emit(typ string, payload any) ([]protocol.Envelope, error) {
	env, err := protocol.NewEnvelope(typ, payload)
	if err != nil {
		return nil, err
	}
	return []protocol.Envelope{env}, nil
}

func sessionToNeutral(s Session) protocol.Session {
	return protocol.Session{
		ID:          s.ID,
		Title:       s.Title,
		Directory:   s.Directory,
		TimeCreated: s.Time.Created,
		TimeUpdated: s.Time.Updated,
	}
}

func messageToNeutral(m Message) protocol.MessageInfo {
	out := protocol.MessageInfo{
		ID:        m.ID,
		SessionID: m.SessionID,
		Role:      m.Role,
		Created:   m.Time.Created,
		Completed: m.Time.Completed,
		Provider:  m.ProviderID,
		Model:     m.ModelID,
		Agent:     m.Agent,
		Cost:      m.Cost,
		Finish:    m.Finish,
	}
	if m.Tokens != nil {
		out.Tokens = &protocol.TokenUsage{
			Input:      m.Tokens.Input,
			Output:     m.Tokens.Output,
			Reasoning:  m.Tokens.Reasoning,
			CacheRead:  m.Tokens.Cache.Read,
			CacheWrite: m.Tokens.Cache.Write,
		}
	}
	if m.Error != nil {
		out.ErrorName = m.Error.Name
		out.ErrorMsg = m.Error.Data.Message
	}
	return out
}

// partToNeutral maps an OpenCode part to the neutral union. The bool result is
// false for part types circuloGo drops in v0 (snapshot/retry/compaction).
func partToNeutral(p Part) (protocol.Part, bool) {
	out := protocol.Part{ID: p.ID, Type: p.Type}
	switch p.Type {
	case "text", "reasoning":
		out.Text = p.Text
		out.Time = timeToNeutral(p.Time)
		if p.Type == "text" && p.Synthetic {
			// Synthetic text parts are server-generated context markers; they
			// render as noise in a transcript. Keep the part (ids must stay
			// stable) but the UI checks synthetic via metadata — v0 drops it.
			return protocol.Part{}, false
		}
	case "tool":
		out.CallID = p.CallID
		out.Tool = p.Tool
		st := &protocol.ToolState{
			Status:   p.State.Status,
			Title:    p.State.Title,
			Output:   p.State.Output,
			Error:    p.State.Error,
			Input:    p.State.Input,
			Metadata: p.State.Metadata,
		}
		if p.State.Time != nil {
			st.Time = &protocol.TimeRange{Start: p.State.Time.Start, End: p.State.Time.End}
		}
		out.State = st
	case "step-start":
		// no extra fields
	case "step-finish":
		out.Reason = p.Reason
		out.Cost = p.Cost
		if p.Tokens != nil {
			out.Tokens = &protocol.TokenUsage{
				Input:      p.Tokens.Input,
				Output:     p.Tokens.Output,
				Reasoning:  p.Tokens.Reasoning,
				CacheRead:  p.Tokens.Cache.Read,
				CacheWrite: p.Tokens.Cache.Write,
			}
		}
	case "patch":
		out.Hash = p.Hash
		out.Files = p.Files
	case "agent":
		out.Name = p.Name
	case "subtask":
		out.Prompt = p.Prompt
		out.Description = p.Description
		out.Agent = p.Agent
	case "file":
		out.Mime = p.Mime
		if p.Source != nil {
			out.Path = p.Source.Path
		}
	case "snapshot", "retry", "compaction":
		return protocol.Part{}, false
	default:
		// Unknown part type: drop rather than emit a part the UI cannot render
		// (NFR-3 tolerates unknown *events*; unknown *parts* have no renderer).
		return protocol.Part{}, false
	}
	return out, true
}

func timeToNeutral(t *struct {
	Start int64 `json:"start,omitempty"`
	End   int64 `json:"end,omitempty"`
}) *protocol.TimeRange {
	if t == nil {
		return nil
	}
	return &protocol.TimeRange{Start: t.Start, End: t.End}
}

// partFieldType maps a part type to the field a delta applies to. Non-text
// parts do not receive deltas on the v1 stream today; "" is the safe default
// that reducers ignore.
func partFieldType(partType string) string {
	switch partType {
	case protocol.PartText:
		return "text"
	case protocol.PartReasoning:
		return "reasoning"
	default:
		return ""
	}
}

// patternsToRaw packs the patterns list into opaque JSON for the UI (v0 renders
// the first pattern as detail text).
func patternsToRaw(patterns []string) json.RawMessage {
	if len(patterns) == 0 {
		return nil
	}
	raw, err := json.Marshal(patterns)
	if err != nil {
		return nil
	}
	return raw
}
