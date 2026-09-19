package opencode

import (
	"os"
	"strings"
	"testing"

	"circulogo/internal/agent/protocol"
)

// fixtureEnvelopes parses every frame of a real-capture fixture and translates
// it, returning the protocol envelopes in stream order. Any frame that fails to
// decode or translate fails the test — real captures must keep flowing through
// the translator unchanged.
func fixtureEnvelopes(t *testing.T, name string) []protocol.Envelope {
	t.Helper()
	raw, err := os.ReadFile("testdata/" + name)
	if err != nil {
		t.Fatal(err)
	}
	var out []protocol.Envelope
	for _, f := range collect(t, strings.NewReader(string(raw))) {
		env, err := parseV2Event(f)
		if err != nil {
			t.Fatalf("decode frame: %v\nframe: %.120s", err, f)
		}
		got, err := TranslateV2("p1", env)
		if err != nil {
			t.Fatalf("translate %s: %v", env.Type, err)
		}
		out = append(out, got...)
	}
	return out
}

func TestTranslateV2_PermissionRoundtripFixture(t *testing.T) {
	envs := fixtureEnvelopes(t, "v2-permission-roundtrip.sse")

	var asked, replied bool
	for _, e := range envs {
		switch e.Type {
		case protocol.EventPermissionRequest:
			if asked {
				t.Fatal("second permission.request in fixture")
			}
			asked = true
			var p protocol.PermissionRequested
			payloadAs(t, e, &p)
			if p.ProjectID != "p1" || p.SessionID != "ses_f4a23cf2affeQb4z18CwGP3HjX" {
				t.Errorf("scoping = %s/%s", p.ProjectID, p.SessionID)
			}
			perm := p.Permission
			if perm.ID != "per_0b5dc4ec4001nJIDWc8MarHiyB" {
				t.Errorf("perm id = %s", perm.ID)
			}
			if perm.Kind != "edit" {
				t.Errorf("kind = %s, want edit (v2 renames bash→shell)", perm.Kind)
			}
			if perm.Title != "edit notes.txt" {
				t.Errorf("title = %q, want action+resources", perm.Title)
			}
			if perm.CallID != "call_00_EEORc4SxiGAvEieYNdVY9907" {
				t.Errorf("callID = %s, want source tool call id", perm.CallID)
			}
			if perm.CreatedAt != 1789757509316 {
				t.Errorf("createdAt = %d, want envelope created", perm.CreatedAt)
			}
			if !strings.Contains(string(perm.Metadata), "notes.txt") {
				t.Errorf("metadata not passed through: %s", perm.Metadata)
			}
		case protocol.EventPermissionResolved:
			if replied {
				t.Fatal("second permission.resolved in fixture")
			}
			replied = true
			var p protocol.PermissionResolved
			payloadAs(t, e, &p)
			if p.PermissionID != "per_0b5dc4ec4001nJIDWc8MarHiyB" {
				t.Errorf("permissionID = %s, want requestID echo", p.PermissionID)
			}
			if p.Response != "once" {
				t.Errorf("response = %s, want reply echo", p.Response)
			}
			if p.SessionID != "ses_f4a23cf2affeQb4z18CwGP3HjX" {
				t.Errorf("sessionID = %s", p.SessionID)
			}
		}
	}
	if !asked || !replied {
		t.Fatalf("fixture must cover ask+reply: asked=%v replied=%v", asked, replied)
	}
}

func TestTranslateV2_ExecutionFailedFixture(t *testing.T) {
	envs := fixtureEnvelopes(t, "v2-execution-failed.sse")

	var sawStatusIdle, sawError bool
	for _, e := range envs {
		if e.Type != protocol.EventSessionStatus && e.Type != protocol.EventSessionError {
			continue
		}
		if e.Type == protocol.EventSessionStatus {
			var s protocol.SessionStatus
			payloadAs(t, e, &s)
			if s.Status == protocol.StatusIdle {
				sawStatusIdle = true
			}
			continue
		}
		sawError = true
		var p protocol.SessionErrored
		payloadAs(t, e, &p)
		if p.SessionID != "ses_f4a221d48ffebrLAC7xXx38llS" {
			t.Errorf("sessionID = %s", p.SessionID)
		}
		if p.Error.Name != "provider.no-route" {
			t.Errorf("error name = %s, want provider.no-route", p.Error.Name)
		}
		if p.Error.Message != "Model unavailable: deepseek/no-such-model-xyz" {
			t.Errorf("error message = %q", p.Error.Message)
		}
	}
	if !sawStatusIdle || !sawError {
		t.Fatalf("failed turn must idle the session AND surface the error: idle=%v err=%v", sawStatusIdle, sawError)
	}
}

// The full 2.0.8 capture set must flow through the translator error-free —
// guards the ignore-list against crashing on known noise.
func TestTranslateV2_AllV2FixturesAreTranslatable(t *testing.T) {
	for _, name := range []string{
		"v2-turn-basic.sse",
		"v2-session-lifecycle.sse",
		"v2-permission-roundtrip.sse",
		"v2-execution-failed.sse",
	} {
		fixtureEnvelopes(t, name)
	}
}
