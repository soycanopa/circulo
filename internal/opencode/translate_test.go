package opencode

import (
	"encoding/json"
	"os"
	"strings"
	"testing"

	"circulogo/internal/agent/protocol"
)

// feedFixture decodes + translates a fixture file and returns every neutral
// envelope produced, in order.
func feedFixture(t *testing.T, name, projectID string) []protocol.Envelope {
	t.Helper()
	raw, err := os.ReadFile("testdata/" + name)
	if err != nil {
		t.Fatal(err)
	}
	frames := collect(t, strings.NewReader(string(raw)))
	var out []protocol.Envelope
	for _, f := range frames {
		env, err := DecodeEvent(f)
		if err != nil {
			t.Fatalf("%s: decode: %v", name, err)
		}
		got, err := Translate(projectID, env)
		if err != nil {
			t.Fatalf("%s: translate %s: %v", name, env.Type, err)
		}
		out = append(out, got...)
	}
	return out
}

func envelopeTypes(envs []protocol.Envelope) []string {
	types := make([]string, len(envs))
	for i, e := range envs {
		types[i] = e.Type
	}
	return types
}

func contains(list []string, want string) bool {
	for _, v := range list {
		if v == want {
			return true
		}
	}
	return false
}

func TestTranslate_TurnBasicFixture(t *testing.T) {
	envs := feedFixture(t, "turn-basic.sse", "proj_test")
	types := envelopeTypes(envs)

	for _, want := range []string{
		protocol.EventAdapterStatus,
		protocol.EventSessionUpdated,
		protocol.EventSessionStatus,
		protocol.EventMessageUpdated,
		protocol.EventPartUpdated,
		protocol.EventPartDelta,
	} {
		if !contains(types, want) {
			t.Errorf("missing neutral event %s\nproduced: %v", want, types)
		}
	}
	// Noise must not leak through the translator.
	for _, banned := range []string{"server.heartbeat", "plugin.added", "catalog.updated"} {
		if contains(types, banned) {
			t.Errorf("noise event %s leaked into neutral stream", banned)
		}
	}
}

func TestTranslate_TurnBasic_PartLifecycle(t *testing.T) {
	envs := feedFixture(t, "turn-basic.sse", "proj_test")

	var textParts []protocol.Part
	var sawStepStart, sawStepFinish bool
	var finishTokens *protocol.TokenUsage
	for _, e := range envs {
		if e.Type != protocol.EventPartUpdated {
			continue
		}
		var p protocol.PartUpdated
		if err := json.Unmarshal(e.Payload, &p); err != nil {
			t.Fatal(err)
		}
		if p.ProjectID != "proj_test" {
			t.Errorf("part.updated projectID = %q", p.ProjectID)
		}
		switch p.Part.Type {
		case protocol.PartText:
			textParts = append(textParts, p.Part)
		case protocol.PartStepStart:
			sawStepStart = true
		case protocol.PartStepFinish:
			sawStepFinish = true
			finishTokens = p.Part.Tokens
		}
	}
	if !sawStepStart || !sawStepFinish {
		t.Fatalf("step markers missing: start=%v finish=%v", sawStepStart, sawStepFinish)
	}
	// Invariant from the live capture: the last text part carries the full
	// accumulated text ("OK"), which is what makes the reducer self-healing.
	last := textParts[len(textParts)-1]
	if last.Text != "OK" {
		t.Errorf("final text = %q, want %q", last.Text, "OK")
	}
	if finishTokens == nil || finishTokens.Output != 3 {
		t.Errorf("step-finish tokens = %+v, want output=3", finishTokens)
	}
}

func TestTranslate_RetryErrorFixture(t *testing.T) {
	envs := feedFixture(t, "retry-error.sse", "proj_test")

	var sawRetry, sawError bool
	for _, e := range envs {
		switch e.Type {
		case protocol.EventSessionStatus:
			var p protocol.SessionStatus
			if err := json.Unmarshal(e.Payload, &p); err != nil {
				t.Fatal(err)
			}
			if p.Status == protocol.StatusRetry {
				sawRetry = true
				if p.Retry == nil || p.Retry.Attempt < 1 || p.Retry.Message == "" {
					t.Errorf("retry payload incomplete: %+v", p.Retry)
				}
			}
		case protocol.EventSessionError:
			var p protocol.SessionErrored
			if err := json.Unmarshal(e.Payload, &p); err != nil {
				t.Fatal(err)
			}
			sawError = true
			if p.Error.Name != "APIError" {
				t.Errorf("error name = %q, want APIError", p.Error.Name)
			}
		}
	}
	if !sawRetry {
		t.Error("fixture produced no retry status")
	}
	if !sawError {
		t.Error("fixture produced no session error")
	}
}

func TestTranslate_PermissionFixture(t *testing.T) {
	envs := feedFixture(t, "permission.sse", "proj_test")
	if len(envs) != 2 {
		t.Fatalf("got %d envelopes, want 2: %v", len(envs), envelopeTypes(envs))
	}
	var req protocol.PermissionRequested
	if err := json.Unmarshal(envs[0].Payload, &req); err != nil {
		t.Fatal(err)
	}
	if req.Permission.Kind != "bash" || req.Permission.ID == "" || req.Permission.CallID == "" {
		t.Errorf("permission request mapped wrong: %+v", req.Permission)
	}
	var res protocol.PermissionResolved
	if err := json.Unmarshal(envs[1].Payload, &res); err != nil {
		t.Fatal(err)
	}
	if res.PermissionID != req.Permission.ID || res.Response != protocol.PermissionOnce {
		t.Errorf("permission resolved mapped wrong: %+v", res)
	}
}

func TestTranslate_UnknownEventsAndPartsIgnored(t *testing.T) {
	// Unknown event type: must translate to zero envelopes, nil error.
	envs, err := Translate("p", Envelope{Type: "future.metric.awesome", Properties: json.RawMessage(`{}`)})
	if err != nil {
		t.Fatalf("unknown event returned error: %v", err)
	}
	if len(envs) != 0 {
		t.Fatalf("unknown event produced %d envelopes", len(envs))
	}

	// Unknown part type inside a known event: dropped.
	props := `{"part":{"id":"prt_x","type":"holo","messageID":"msg_x","sessionID":"ses_x"}}`
	envs, err = Translate("p", Envelope{Type: "message.part.updated", Properties: json.RawMessage(props)})
	if err != nil {
		t.Fatalf("unknown part returned error: %v", err)
	}
	if len(envs) != 0 {
		t.Fatalf("unknown part produced %d envelopes", len(envs))
	}
}

func TestTranslate_BadPropertiesIsError(t *testing.T) {
	// Corrupt properties on a KNOWN event type is a real protocol error and
	// must surface (unlike unknown types, which are ignored).
	_, err := Translate("p", Envelope{Type: "session.status", Properties: json.RawMessage(`{"sessionID": 12`)})
	if err == nil {
		t.Fatal("expected error for corrupt properties")
	}
}
