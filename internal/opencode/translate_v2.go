package opencode

import (
	"encoding/json"
	"fmt"
	"strings"
	"time"

	"circulogo/internal/agent/protocol"
)

// TranslateV2 converts one OpenCode v2 event into zero or more neutral
// protocol envelopes. Zero results are the normal path for the ignore-list
// (config-refresh noise, inbox delivery internals) and for unknown future
// types — they degrade to "ignored", never to errors.
//
// v2 streams reasoning/text as started→delta→ended with cumulative text on
// the *ended frame; the neutral contract keeps v1's replace semantics, so
// started creates an empty part, deltas append (paint hint) and ended
// replaces with the authoritative full text. Part ids are synthesized
// `<assistantMessageID>:text|:reasoning` so live events and REST hydration
// land on the same parts; tool parts use the provider's call id.
func TranslateV2(projectID string, env V2Event) ([]protocol.Envelope, error) {
	switch env.Type {
	case "server.connected":
		return emit(protocol.EventAdapterStatus, protocol.AdapterStatus{
			ProjectID: projectID,
			State:     protocol.AdapterRunning,
		})

	case "session.created":
		var p struct {
			SessionID string `json:"sessionID"`
			Location  struct {
				Directory string `json:"directory"`
			} `json:"location"`
		}
		if err := json.Unmarshal(env.Data, &p); err != nil {
			return nil, fmt.Errorf("opencode: session.created: %w", err)
		}
		return emit(protocol.EventSessionUpdated, protocol.SessionUpdated{
			ProjectID: projectID,
			Session: protocol.Session{
				ID:          p.SessionID,
				Directory:   p.Location.Directory,
				TimeCreated: orNow(env.Created),
				TimeUpdated: orNow(env.Created),
			},
		})

	case "session.renamed":
		var p struct {
			SessionID string `json:"sessionID"`
			Title     string `json:"title"`
		}
		if err := json.Unmarshal(env.Data, &p); err != nil {
			return nil, fmt.Errorf("opencode: session.renamed: %w", err)
		}
		// Partial patch: the reducer keeps existing title/times for known
		// sessions and skips unknown ids (this payload has no directory, so
		// scoping already happened or the event is dropped upstream).
		return emit(protocol.EventSessionUpdated, protocol.SessionUpdated{
			ProjectID: projectID,
			Session: protocol.Session{
				ID:    p.SessionID,
				Title: p.Title,
			},
		})

	case "session.execution.started":
		var p struct {
			SessionID string `json:"sessionID"`
		}
		if err := json.Unmarshal(env.Data, &p); err != nil {
			return nil, fmt.Errorf("opencode: session.execution.started: %w", err)
		}
		return emit(protocol.EventSessionStatus, protocol.SessionStatus{
			ProjectID: projectID,
			SessionID: p.SessionID,
			Status:    protocol.StatusBusy,
		})

	case "session.execution.succeeded":
		var p struct {
			SessionID string `json:"sessionID"`
		}
		if err := json.Unmarshal(env.Data, &p); err != nil {
			return nil, fmt.Errorf("opencode: session.execution.succeeded: %w", err)
		}
		return emit(protocol.EventSessionStatus, protocol.SessionStatus{
			ProjectID: projectID,
			SessionID: p.SessionID,
			Status:    protocol.StatusIdle,
		})

	case "session.execution.failed":
		// Live shape (v2-execution-failed.sse): {sessionID, error:{type,
		// message}} — `status` only appears for HTTP-backed errors.
		var p struct {
			SessionID string `json:"sessionID"`
			Error     *struct {
				Type    string `json:"type"`
				Message string `json:"message"`
				Status  int    `json:"status"`
			} `json:"error"`
		}
		if err := json.Unmarshal(env.Data, &p); err != nil {
			return nil, fmt.Errorf("opencode: session.execution.failed: %w", err)
		}
		out := []protocol.Envelope{}
		idle, err := emit(protocol.EventSessionStatus, protocol.SessionStatus{
			ProjectID: projectID,
			SessionID: p.SessionID,
			Status:    protocol.StatusIdle,
		})
		if err != nil {
			return nil, err
		}
		out = append(out, idle...)
		if p.Error != nil {
			e, err := emit(protocol.EventSessionError, protocol.SessionErrored{
				ProjectID: projectID,
				SessionID: p.SessionID,
				Error: protocol.AgentError{
					Name:    p.Error.Type,
					Message: p.Error.Message,
				},
			})
			if err != nil {
				return nil, err
			}
			out = append(out, e...)
		}
		return out, nil

	case "session.inbox.enqueued":
		// The user message rides the inbox: item.type=="user" carries the
		// text. Emitting it as message.updated + text part lets the reducer's
		// optimistic dedupe replace the client_ bubble with the server echo.
		var p struct {
			InboxID   string `json:"inboxID"`
			SessionID string `json:"sessionID"`
			Item      struct {
				Type    string `json:"type"`
				Payload struct {
					Text string `json:"text"`
				} `json:"payload"`
			} `json:"item"`
		}
		if err := json.Unmarshal(env.Data, &p); err != nil {
			return nil, fmt.Errorf("opencode: session.inbox.enqueued: %w", err)
		}
		if p.Item.Type != "user" {
			return nil, nil
		}
		created := orNow(env.Created)
		msg, err := emit(protocol.EventMessageUpdated, protocol.MessageUpdated{
			ProjectID: projectID,
			SessionID: p.SessionID,
			Message: protocol.MessageInfo{
				ID:        p.InboxID,
				SessionID: p.SessionID,
				Role:      protocol.RoleUser,
				Created:   created,
			},
		})
		if err != nil {
			return nil, err
		}
		part, err := emit(protocol.EventPartUpdated, protocol.PartUpdated{
			ProjectID: projectID,
			SessionID: p.SessionID,
			MessageID: p.InboxID,
			Part: protocol.Part{
				ID:   p.InboxID + ":text",
				Type: protocol.PartText,
				Text: p.Item.Payload.Text,
			},
		})
		if err != nil {
			return nil, err
		}
		return append(msg, part...), nil

	case "session.text.started", "session.reasoning.started":
		var p struct {
			SessionID          string `json:"sessionID"`
			AssistantMessageID string `json:"assistantMessageID"`
		}
		if err := json.Unmarshal(env.Data, &p); err != nil {
			return nil, fmt.Errorf("opencode: %s: %w", env.Type, err)
		}
		partType, partID := streamPart(p.AssistantMessageID, env.Type)
		out := []protocol.Envelope{}
		msg, err := emit(protocol.EventMessageUpdated, protocol.MessageUpdated{
			ProjectID: projectID,
			SessionID: p.SessionID,
			Message: protocol.MessageInfo{
				ID:        p.AssistantMessageID,
				SessionID: p.SessionID,
				Role:      protocol.RoleAssistant,
				Created:   orNow(env.Created),
			},
		})
		if err != nil {
			return nil, err
		}
		out = append(out, msg...)
		part, err := emit(protocol.EventPartUpdated, protocol.PartUpdated{
			ProjectID: projectID,
			SessionID: p.SessionID,
			MessageID: p.AssistantMessageID,
			Part: protocol.Part{
				ID:   partID,
				Type: partType,
			},
		})
		if err != nil {
			return nil, err
		}
		return append(out, part...), nil

	case "session.text.delta", "session.reasoning.delta":
		var p struct {
			SessionID          string `json:"sessionID"`
			AssistantMessageID string `json:"assistantMessageID"`
			Delta              string `json:"delta"`
		}
		if err := json.Unmarshal(env.Data, &p); err != nil {
			return nil, fmt.Errorf("opencode: %s: %w", env.Type, err)
		}
		partType, partID := streamPart(p.AssistantMessageID, env.Type)
		return emit(protocol.EventPartDelta, protocol.PartDelta{
			ProjectID: projectID,
			SessionID: p.SessionID,
			MessageID: p.AssistantMessageID,
			PartID:    partID,
			Field:     partType,
			Delta:     p.Delta,
		})

	case "session.text.ended", "session.reasoning.ended":
		var p struct {
			SessionID          string `json:"sessionID"`
			AssistantMessageID string `json:"assistantMessageID"`
			Text               string `json:"text"`
		}
		if err := json.Unmarshal(env.Data, &p); err != nil {
			return nil, fmt.Errorf("opencode: %s: %w", env.Type, err)
		}
		partType, partID := streamPart(p.AssistantMessageID, env.Type)
		return emit(protocol.EventPartUpdated, protocol.PartUpdated{
			ProjectID: projectID,
			SessionID: p.SessionID,
			MessageID: p.AssistantMessageID,
			Part: protocol.Part{
				ID:   partID,
				Type: partType,
				Text: p.Text,
				Time: &protocol.TimeRange{Start: orNow(env.Created)},
			},
		})

	case "session.tool.input.started":
		var p struct {
			SessionID          string `json:"sessionID"`
			AssistantMessageID string `json:"assistantMessageID"`
			ID                 string `json:"id"`
			Name               string `json:"name"`
		}
		if err := json.Unmarshal(env.Data, &p); err != nil {
			return nil, fmt.Errorf("opencode: session.tool.input.started: %w", err)
		}
		out := []protocol.Envelope{}
		msg, err := emit(protocol.EventMessageUpdated, protocol.MessageUpdated{
			ProjectID: projectID,
			SessionID: p.SessionID,
			Message: protocol.MessageInfo{
				ID:        p.AssistantMessageID,
				SessionID: p.SessionID,
				Role:      protocol.RoleAssistant,
				Created:   orNow(env.Created),
			},
		})
		if err != nil {
			return nil, err
		}
		out = append(out, msg...)
		part, err := emit(protocol.EventPartUpdated, protocol.PartUpdated{
			ProjectID: projectID,
			SessionID: p.SessionID,
			MessageID: p.AssistantMessageID,
			Part: protocol.Part{
				ID:     p.ID,
				Type:   protocol.PartTool,
				CallID: p.ID,
				Tool:   p.Name,
				State:  &protocol.ToolState{Status: protocol.ToolPending},
			},
		})
		if err != nil {
			return nil, err
		}
		return append(out, part...), nil

	case "session.tool.input.ended":
		var p struct {
			SessionID          string `json:"sessionID"`
			AssistantMessageID string `json:"assistantMessageID"`
			ID                 string `json:"id"`
			Text               string `json:"text"`
		}
		if err := json.Unmarshal(env.Data, &p); err != nil {
			return nil, fmt.Errorf("opencode: session.tool.input.ended: %w", err)
		}
		return emit(protocol.EventPartUpdated, protocol.PartUpdated{
			ProjectID: projectID,
			SessionID: p.SessionID,
			MessageID: p.AssistantMessageID,
			Part: protocol.Part{
				ID:   p.ID,
				Type: protocol.PartTool,
				State: &protocol.ToolState{
					Status: protocol.ToolRunning,
					Input:  json.RawMessage(p.Text),
				},
			},
		})

	case "session.tool.called":
		var p struct {
			SessionID          string          `json:"sessionID"`
			AssistantMessageID string          `json:"assistantMessageID"`
			ID                 string          `json:"id"`
			Input              json.RawMessage `json:"input"`
		}
		if err := json.Unmarshal(env.Data, &p); err != nil {
			return nil, fmt.Errorf("opencode: session.tool.called: %w", err)
		}
		return emit(protocol.EventPartUpdated, protocol.PartUpdated{
			ProjectID: projectID,
			SessionID: p.SessionID,
			MessageID: p.AssistantMessageID,
			Part: protocol.Part{
				ID:   p.ID,
				Type: protocol.PartTool,
				State: &protocol.ToolState{
					Status: protocol.ToolRunning,
					Input:  p.Input,
				},
			},
		})

	case "session.tool.progress":
		var p struct {
			SessionID          string          `json:"sessionID"`
			AssistantMessageID string          `json:"assistantMessageID"`
			ID                 string          `json:"id"`
			Metadata           json.RawMessage `json:"metadata"`
		}
		if err := json.Unmarshal(env.Data, &p); err != nil {
			return nil, fmt.Errorf("opencode: session.tool.progress: %w", err)
		}
		return emit(protocol.EventPartUpdated, protocol.PartUpdated{
			ProjectID: projectID,
			SessionID: p.SessionID,
			MessageID: p.AssistantMessageID,
			Part: protocol.Part{
				ID:   p.ID,
				Type: protocol.PartTool,
				State: &protocol.ToolState{
					Status:   protocol.ToolRunning,
					Metadata: p.Metadata,
				},
			},
		})

	case "session.tool.success":
		var p struct {
			SessionID          string `json:"sessionID"`
			AssistantMessageID string `json:"assistantMessageID"`
			ID                 string `json:"id"`
			Content            []struct {
				Type string `json:"type"`
				Text string `json:"text"`
			} `json:"content"`
		}
		if err := json.Unmarshal(env.Data, &p); err != nil {
			return nil, fmt.Errorf("opencode: session.tool.success: %w", err)
		}
		var b strings.Builder
		for _, c := range p.Content {
			if c.Text != "" {
				if b.Len() > 0 {
					b.WriteString("\n")
				}
				b.WriteString(c.Text)
			}
		}
		return emit(protocol.EventPartUpdated, protocol.PartUpdated{
			ProjectID: projectID,
			SessionID: p.SessionID,
			MessageID: p.AssistantMessageID,
			Part: protocol.Part{
				ID:   p.ID,
				Type: protocol.PartTool,
				State: &protocol.ToolState{
					Status: protocol.ToolCompleted,
					Output: b.String(),
				},
			},
		})

	case "session.tool.error":
		// Event name not yet captured live; the error rides either as a
		// string or the StructuredError object the REST states use — accept
		// both, a rigid decode would silently drop the frame.
		var p struct {
			SessionID          string          `json:"sessionID"`
			AssistantMessageID string          `json:"assistantMessageID"`
			ID                 string          `json:"id"`
			Error              json.RawMessage `json:"error"`
		}
		if err := json.Unmarshal(env.Data, &p); err != nil {
			return nil, fmt.Errorf("opencode: session.tool.error: %w", err)
		}
		return emit(protocol.EventPartUpdated, protocol.PartUpdated{
			ProjectID: projectID,
			SessionID: p.SessionID,
			MessageID: p.AssistantMessageID,
			Part: protocol.Part{
				ID:   p.ID,
				Type: protocol.PartTool,
				State: &protocol.ToolState{
					Status: protocol.ToolError,
					Error:  rawErrorText(p.Error),
				},
			},
		})

	case "permission.asked":
		// Event name is capture-only (spec/docs don't name SSE events); data
		// shape matches the spec's Permission.Request: {id, sessionID,
		// action, resources, save, metadata, source} (v2-permission-
		// roundtrip.sse). Docs default unmatched tools to ask; the reply
		// vocabulary is once|always|reject.
		var p struct {
			ID        string          `json:"id"`
			SessionID string          `json:"sessionID"`
			Action    string          `json:"action"`
			Resources []string        `json:"resources"`
			Metadata  json.RawMessage `json:"metadata"`
			Source    *struct {
				ID string `json:"id"`
			} `json:"source"`
		}
		if err := json.Unmarshal(env.Data, &p); err != nil {
			return nil, fmt.Errorf("opencode: permission.asked: %w", err)
		}
		perm := protocol.PermissionRequest{
			ID:        p.ID,
			Kind:      p.Action,
			Metadata:  p.Metadata,
			CreatedAt: orNow(env.Created),
		}
		if p.Source != nil {
			perm.CallID = p.Source.ID
		}
		if len(p.Resources) > 0 {
			// Human-readable title from wire data (action + resources); the
			// resource list rides Pattern, the neutral opaque slot.
			perm.Title = p.Action + " " + strings.Join(p.Resources, ", ")
			if raw, err := json.Marshal(p.Resources); err == nil {
				perm.Pattern = raw
			}
		}
		return emit(protocol.EventPermissionRequest, protocol.PermissionRequested{
			ProjectID:  projectID,
			SessionID:  p.SessionID,
			Permission: perm,
		})

	case "permission.replied":
		// Broadcast after the reply POST resolves; the reducer clears the
		// card by permissionID. `reject` cascades to the session's other
		// pending asks (docs/permissions), each with its own replied event.
		var p struct {
			SessionID string `json:"sessionID"`
			RequestID string `json:"requestID"`
			Reply     string `json:"reply"`
		}
		if err := json.Unmarshal(env.Data, &p); err != nil {
			return nil, fmt.Errorf("opencode: permission.replied: %w", err)
		}
		return emit(protocol.EventPermissionResolved, protocol.PermissionResolved{
			ProjectID:    projectID,
			SessionID:    p.SessionID,
			PermissionID: p.RequestID,
			Response:     p.Reply,
		})

	case "form.created":
		// Live shape (q-probe capture, 2.0.8): the question tool blocks by
		// creating a form — {form: {id, sessionID, title, metadata:{kind,
		// tool:{messageID,id}}, fields:[{key,title,description,type,options,
		// custom}]}}. Surfaces as a neutral pending form for the composer.
		var p struct {
			Form struct {
				ID        string `json:"id"`
				SessionID string `json:"sessionID"`
				Title     string `json:"title"`
				Metadata  struct {
					Kind string `json:"kind"`
					Tool struct {
						ID string `json:"id"`
					} `json:"tool"`
				} `json:"metadata"`
				Fields []struct {
					Key         string `json:"key"`
					Title       string `json:"title"`
					Description string `json:"description"`
					Type        string `json:"type"`
					Custom      bool   `json:"custom"`
					Options     []struct {
						Value       string `json:"value"`
						Label       string `json:"label"`
						Description string `json:"description"`
					} `json:"options"`
				} `json:"fields"`
			} `json:"form"`
		}
		if err := json.Unmarshal(env.Data, &p); err != nil {
			return nil, fmt.Errorf("opencode: form.created: %w", err)
		}
		form := protocol.Form{
			ID:     p.Form.ID,
			Title:  p.Form.Title,
			Kind:   p.Form.Metadata.Kind,
			CallID: p.Form.Metadata.Tool.ID,
		}
		for _, f := range p.Form.Fields {
			// V0 renders select-style and free-text fields; numeric/bool
			// variants degrade to text (the question tool emits strings).
			field := protocol.FormField{
				Key:         f.Key,
				Title:       f.Title,
				Description: f.Description,
				Custom:      f.Custom,
			}
			for _, o := range f.Options {
				field.Options = append(field.Options, protocol.FormOption{
					Value: o.Value, Label: o.Label, Description: o.Description,
				})
			}
			form.Fields = append(form.Fields, field)
		}
		return emit(protocol.EventFormUpdated, protocol.FormUpdated{
			ProjectID: projectID,
			SessionID: p.Form.SessionID,
			Form:      form,
		})

	case "form.replied":
		// {id, sessionID, answer} — the answer is visible in the transcript
		// (tool output); the card just needs to close.
		var p struct {
			ID        string `json:"id"`
			SessionID string `json:"sessionID"`
		}
		if err := json.Unmarshal(env.Data, &p); err != nil {
			return nil, fmt.Errorf("opencode: form.replied: %w", err)
		}
		return emit(protocol.EventFormResolved, protocol.FormResolved{
			ProjectID: projectID,
			SessionID: p.SessionID,
			FormID:    p.ID,
		})

	case "session.step.started":
		var p struct {
			SessionID          string `json:"sessionID"`
			AssistantMessageID string `json:"assistantMessageID"`
			Agent              string `json:"agent"`
			Model              struct {
				ID         string `json:"id"`
				ProviderID string `json:"providerID"`
			} `json:"model"`
		}
		if err := json.Unmarshal(env.Data, &p); err != nil {
			return nil, fmt.Errorf("opencode: session.step.started: %w", err)
		}
		return emit(protocol.EventMessageUpdated, protocol.MessageUpdated{
			ProjectID: projectID,
			SessionID: p.SessionID,
			Message: protocol.MessageInfo{
				ID:        p.AssistantMessageID,
				SessionID: p.SessionID,
				Role:      protocol.RoleAssistant,
				Agent:     p.Agent,
				Provider:  p.Model.ProviderID,
				Model:     p.Model.ID,
				Created:   orNow(env.Created),
			},
		})

	case "session.step.ended":
		var p struct {
			SessionID          string  `json:"sessionID"`
			AssistantMessageID string  `json:"assistantMessageID"`
			Finish             string  `json:"finish"`
			Cost               float64 `json:"cost"`
			Tokens             *struct {
				Input     int64 `json:"input"`
				Output    int64 `json:"output"`
				Reasoning int64 `json:"reasoning"`
				Cache     struct {
					Read  int64 `json:"read"`
					Write int64 `json:"write"`
				} `json:"cache"`
			} `json:"tokens"`
		}
		if err := json.Unmarshal(env.Data, &p); err != nil {
			return nil, fmt.Errorf("opencode: session.step.ended: %w", err)
		}
		part := protocol.Part{
			ID:     p.AssistantMessageID + ":finish",
			Type:   protocol.PartStepFinish,
			Reason: p.Finish,
			Cost:   p.Cost,
		}
		if p.Tokens != nil {
			part.Tokens = &protocol.TokenUsage{
				Input:      p.Tokens.Input,
				Output:     p.Tokens.Output,
				Reasoning:  p.Tokens.Reasoning,
				CacheRead:  p.Tokens.Cache.Read,
				CacheWrite: p.Tokens.Cache.Write,
			}
		}
		return emit(protocol.EventPartUpdated, protocol.PartUpdated{
			ProjectID: projectID,
			SessionID: p.SessionID,
			MessageID: p.AssistantMessageID,
			Part:      part,
		})
	}

	// Ignore-list: config refresh noise (model/provider/command/skill/
	// websearch/instructions .updated), MCP status, integration updates,
	// shell lifecycle, inbox delivery internals, session.usage.updated
	// (session totals; step.ended already carries the per-turn numbers) and
	// anything unknown — dropped by design, never an error.
	return nil, nil
}

func emit(typ string, payload any) ([]protocol.Envelope, error) {
	env, err := protocol.NewEnvelope(typ, payload)
	if err != nil {
		return nil, err
	}
	return []protocol.Envelope{env}, nil
}

// streamPart maps a v2 stream event to the neutral part type and its
// synthesized stable part id.
func streamPart(assistantMessageID, eventType string) (partType, partID string) {
	if strings.Contains(eventType, "reasoning") {
		return protocol.PartReasoning, assistantMessageID + ":reasoning"
	}
	return protocol.PartText, assistantMessageID + ":text"
}

// orNow returns ms if non-zero, else the current time — v2 envelopes carry
// created on most frames but not all.
func orNow(ms int64) int64 {
	if ms != 0 {
		return ms
	}
	return time.Now().UnixMilli()
}
