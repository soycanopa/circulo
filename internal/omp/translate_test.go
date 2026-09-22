package omp

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"circulogo/internal/agent/protocol"
)

// fixtureFrames reads a testdata fixture, skipping '#' comment lines.
func fixtureFrames(t *testing.T, name string) []json.RawMessage {
	t.Helper()
	raw, err := os.ReadFile(filepath.Join("testdata", name))
	if err != nil {
		t.Fatal(err)
	}
	var out []json.RawMessage
	for _, line := range strings.Split(string(raw), "\n") {
		line = strings.TrimSpace(line)
		if line == "" || line[0] == '#' {
			continue
		}
		out = append(out, json.RawMessage(line))
	}
	return out
}

func frameKindOf(t *testing.T, raw json.RawMessage) frameType {
	t.Helper()
	var e envelope
	if err := json.Unmarshal(raw, &e); err != nil {
		t.Fatal(err)
	}
	return e.Type
}

func TestTranslateBasicTurnFixture(t *testing.T) {
	tr := newTranslator("p1")
	var (
		gotStatus   []string
		textDeltas  int
		reasonDelta int
		sawFinish   bool
		msgIDs      = map[string]bool{}
	)
	for _, raw := range fixtureFrames(t, "turn-basic.jsonl") {
		typ := frameKindOf(t, raw)
		envs, err := tr.translate(raw, typ, "s1")
		if err != nil {
			t.Fatalf("%s: %v", typ, err)
		}
		for _, env := range envs {
			switch env.Type {
			case protocol.EventSessionStatus:
				var s protocol.SessionStatus
				json.Unmarshal(env.Payload, &s)
				if s.ProjectID != "p1" || s.SessionID != "s1" {
					t.Fatalf("status not stamped: %+v", s)
				}
				gotStatus = append(gotStatus, s.Status)
			case protocol.EventPartDelta:
				var d protocol.PartDelta
				json.Unmarshal(env.Payload, &d)
				if d.Field == "text" {
					textDeltas++
				}
				if d.Field == "reasoning" {
					reasonDelta++
				}
			case protocol.EventPartUpdated:
				var u protocol.PartUpdated
				json.Unmarshal(env.Payload, &u)
				if u.Part.Type == protocol.PartStepFinish {
					sawFinish = true
					if u.Part.Cost <= 0 {
						t.Fatal("finish part missing cost")
					}
				}
				msgIDs[u.Part.ID] = true
			}
		}
	}
	if len(gotStatus) < 2 || gotStatus[0] != protocol.StatusBusy || gotStatus[len(gotStatus)-1] != protocol.StatusIdle {
		t.Fatalf("status sequence = %v", gotStatus)
	}
	if textDeltas == 0 || reasonDelta == 0 {
		t.Fatalf("expected text and reasoning deltas, got %d/%d", textDeltas, reasonDelta)
	}
	if !sawFinish {
		t.Fatal("expected step-finish part")
	}
	// part ids follow the <msgID>:text / :reasoning scheme
	var sawText, sawReasoning bool
	for id := range msgIDs {
		if strings.HasSuffix(id, ":text") {
			sawText = true
		}
		if strings.HasSuffix(id, ":reasoning") {
			sawReasoning = true
		}
	}
	if !sawText || !sawReasoning {
		t.Fatalf("missing suffixed part ids: %v", msgIDs)
	}
}

func TestTranslateToolTurnFixture(t *testing.T) {
	tr := newTranslator("p1")
	toolParts := map[string]protocol.Part{}
	for _, raw := range fixtureFrames(t, "turn-tool.jsonl") {
		typ := frameKindOf(t, raw)
		envs, err := tr.translate(raw, typ, "s1")
		if err != nil {
			t.Fatalf("%s: %v", typ, err)
		}
		for _, env := range envs {
			if env.Type != protocol.EventPartUpdated {
				continue
			}
			var u protocol.PartUpdated
			json.Unmarshal(env.Payload, &u)
			if u.Part.Type == protocol.PartTool {
				toolParts[u.Part.CallID] = u.Part
			}
		}
	}
	if len(toolParts) != 1 {
		t.Fatalf("want exactly one tool call, got %d", len(toolParts))
	}
	for _, p := range toolParts {
		if p.Tool != "bash" {
			t.Fatalf("tool = %q", p.Tool)
		}
		if p.State.Status != protocol.ToolCompleted {
			t.Fatalf("final status = %q", p.State.Status)
		}
		if !strings.Contains(p.State.Output, "circulo-tool-smoke-7") {
			t.Fatalf("output missing tool result: %q", p.State.Output)
		}
		var input struct {
			Command string `json:"command"`
		}
		if json.Unmarshal(p.State.Input, &input) != nil || input.Command == "" {
			t.Fatalf("tool input not captured: %s", string(p.State.Input))
		}
	}
}

func TestTranslateToolPartsCarryAssistantMessageID(t *testing.T) {
	// The reducer keys parts by (sessionID, messageID, partID): tool updates
	// must name the assistant message that issued the call, even though omp's
	// tool frames do not name it. Hydration locates the owning assistant by
	// the same rule, so derive the expectation from it.
	frames := fixtureFrames(t, "turn-tool.jsonl")

	tr := newTranslator("p1")
	var toolMessageID string
	for _, raw := range frames {
		typ := frameKindOf(t, raw)
		envs, err := tr.translate(raw, typ, "s1")
		if err != nil {
			t.Fatal(err)
		}
		for _, env := range envs {
			if env.Type != protocol.EventPartUpdated {
				continue
			}
			var u protocol.PartUpdated
			json.Unmarshal(env.Payload, &u)
			if u.Part.Type == protocol.PartTool {
				toolMessageID = u.MessageID
			}
		}
	}

	var msgs []agentMessage
	for _, raw := range frames {
		var f struct {
			Type    string       `json:"type"`
			Message agentMessage `json:"message"`
		}
		if json.Unmarshal(raw, &f) != nil || f.Type != "message_end" || f.Message.Role == "" {
			continue
		}
		msgs = append(msgs, f.Message)
	}
	hydrated, err := hydrateMessages("s1", msgs, 0)
	if err != nil {
		t.Fatal(err)
	}
	var owningAssistant string
	for _, hm := range hydrated {
		for _, p := range hm.Parts {
			if p.Type == protocol.PartTool {
				owningAssistant = hm.Info.ID
			}
		}
	}
	if toolMessageID == "" || toolMessageID != owningAssistant {
		t.Fatalf("tool part messageID %q != owning assistant %q", toolMessageID, owningAssistant)
	}
}

func TestTranslateExtensionUISelect(t *testing.T) {
	tr := newTranslator("p1")
	raw := json.RawMessage(`{"type":"extension_ui_request","id":"ui_7","method":"select","title":"Pick one","options":["alpha","beta"],"optionDetails":[{"description":"first"},{}]}`)
	envs, err := tr.translate(raw, frameExtensionUIRequest, "s1")
	if err != nil {
		t.Fatal(err)
	}
	if len(envs) != 1 || envs[0].Type != protocol.EventFormUpdated {
		t.Fatalf("unexpected envelopes: %v", envs)
	}
	var fu protocol.FormUpdated
	json.Unmarshal(envs[0].Payload, &fu)
	if fu.Form.Kind != "omp:select" || fu.Form.ID != "ui_7" {
		t.Fatalf("form = %+v", fu.Form)
	}
	opts := fu.Form.Fields[0].Options
	if len(opts) != 2 || opts[0].Description != "first" {
		t.Fatalf("options = %+v", opts)
	}

	// presentation-only methods produce nothing
	raw = json.RawMessage(`{"type":"extension_ui_request","id":"w","method":"setWidget","widgetKey":"k","widgetLines":["x"]}`)
	if envs, err = tr.translate(raw, frameExtensionUIRequest, "s1"); err != nil || len(envs) != 0 {
		t.Fatalf("setWidget should drop, got %v err=%v", envs, err)
	}
}

func TestTranslateDropsUnknownFrames(t *testing.T) {
	tr := newTranslator("p1")
	for _, raw := range []json.RawMessage{
		json.RawMessage(`{"type":"advisor_cost_changed"}`),
		json.RawMessage(`{"type":"ttsr_triggered","rule":"x"}`),
		json.RawMessage(`{"type":"something_new_in_a_future_omp"}`),
	} {
		envs, err := tr.translate(raw, frameKindOf(t, raw), "s1")
		if err != nil || len(envs) != 0 {
			t.Fatalf("unknown frame %s should drop silently, got %v err %v", raw, envs, err)
		}
	}
}

func TestTranslateRetryStatus(t *testing.T) {
	tr := newTranslator("p1")
	envs, err := tr.translate(json.RawMessage(`{"type":"auto_retry_start","attempt":2,"message":"rate limited"}`), frameAutoRetryStart, "s1")
	if err != nil {
		t.Fatal(err)
	}
	var retrying protocol.SessionStatus
	json.Unmarshal(envs[0].Payload, &retrying)
	if retrying.Status != protocol.StatusRetry || retrying.Retry == nil || retrying.Retry.Attempt != 2 {
		t.Fatalf("retry status = %+v", retrying)
	}
	envs, err = tr.translate(json.RawMessage(`{"type":"auto_retry_end"}`), frameAutoRetryEnd, "s1")
	if err != nil {
		t.Fatal(err)
	}
	var busy protocol.SessionStatus
	json.Unmarshal(envs[0].Payload, &busy)
	if busy.Status != protocol.StatusBusy || busy.Retry != nil {
		t.Fatalf("post-retry status = %+v", busy)
	}
}

// TestHydrationMatchesLivePartIDs is the self-healing invariant: the ids the
// live translator emits must equal the ids hydration derives from get_messages
// (docs/trd.md §3.3). It replays the tool fixture both ways.
func TestHydrationMatchesLivePartIDs(t *testing.T) {
	frames := fixtureFrames(t, "turn-tool.jsonl")

	// Live path: collect part ids per message from the translator.
	tr := newTranslator("p1")
	live := map[string]map[string]bool{} // messageID -> partID -> true
	for _, raw := range frames {
		typ := frameKindOf(t, raw)
		envs, err := tr.translate(raw, typ, "s1")
		if err != nil {
			t.Fatal(err)
		}
		for _, env := range envs {
			if env.Type != protocol.EventPartUpdated {
				continue
			}
			var u protocol.PartUpdated
			json.Unmarshal(env.Payload, &u)
			if live[u.MessageID] == nil {
				live[u.MessageID] = map[string]bool{}
			}
			live[u.MessageID][u.Part.ID] = true
		}
	}

	// Hydration path: reconstruct the get_messages payload from the frames'
	// full message objects (what the omp child would return).
	var msgs []agentMessage
	for _, raw := range frames {
		var f struct {
			Type    string       `json:"type"`
			Message agentMessage `json:"message"`
		}
		if json.Unmarshal(raw, &f) != nil || f.Type != "message_end" {
			continue
		}
		if f.Message.Role == "user" || f.Message.Role == "assistant" || f.Message.Role == "toolResult" {
			msgs = append(msgs, f.Message)
		}
	}
	hydrated, err := hydrateMessages("s1", msgs, 0)
	if err != nil {
		t.Fatal(err)
	}
	got := map[string]map[string]bool{}
	for _, hm := range hydrated {
		got[hm.Info.ID] = map[string]bool{}
		for _, p := range hm.Parts {
			got[hm.Info.ID][p.ID] = true
		}
	}

	for msgID, parts := range live {
		if len(parts) == 0 {
			continue
		}
		if got[msgID] == nil {
			t.Fatalf("live message %s missing from hydration", msgID)
		}
		for partID := range parts {
			if !got[msgID][partID] {
				t.Fatalf("live part %s/%s missing from hydration (have %v)", msgID, partID, got[msgID])
			}
		}
	}

	// The tool part must land on the assistant message with result state.
	for _, hm := range hydrated {
		if hm.Info.Role != protocol.RoleAssistant {
			continue
		}
		for _, p := range hm.Parts {
			if p.Type == protocol.PartTool {
				if p.State.Status != protocol.ToolCompleted {
					t.Fatalf("hydrated tool state = %+v", p.State)
				}
				if !strings.Contains(p.State.Output, "circulo-tool-smoke-7") {
					t.Fatalf("hydrated tool output = %q", p.State.Output)
				}
				if p.State.Time == nil || p.State.Time.End == 0 {
					t.Fatalf("hydrated tool time = %+v", p.State.Time)
				}
			}
		}
	}
}

func TestHydrateLimitKeepsTail(t *testing.T) {
	msgs := []agentMessage{
		{Role: "user", Content: json.RawMessage(`"one"`), Timestamp: 1},
		{Role: "user", Content: json.RawMessage(`"two"`), Timestamp: 2},
		{Role: "user", Content: json.RawMessage(`"three"`), Timestamp: 3},
	}
	out, err := hydrateMessages("s1", msgs, 2)
	if err != nil {
		t.Fatal(err)
	}
	if len(out) != 2 || out[0].Parts[0].Text != "two" || out[1].Parts[0].Text != "three" {
		t.Fatalf("limit tail wrong: %+v", out)
	}
}

func TestMessageIDsStablePerSequence(t *testing.T) {
	ids := newMessageIDs()
	if got := ids.next("user"); got != "u1" {
		t.Fatalf("first user id = %q", got)
	}
	if got := ids.next("assistant"); got != "a1" {
		t.Fatalf("first assistant id = %q", got)
	}
	if got := ids.next("assistant"); got != "a2" {
		t.Fatalf("second assistant id = %q", got)
	}
	if got := ids.next("user"); got != "u2" {
		t.Fatalf("second user id = %q", got)
	}
	// A fresh sequencer (hydration path, adapter restart) must reproduce the
	// same ids from the same sequence.
	fresh := newMessageIDs()
	fresh.next("user")
	if got := fresh.next("assistant"); got != "a1" {
		t.Fatalf("fresh sequencer assistant id = %q", got)
	}
}
