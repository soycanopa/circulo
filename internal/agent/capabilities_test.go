package agent_test

import (
	"encoding/json"
	"testing"

	"circulogo/internal/agent/protocol"
	"circulogo/internal/omp"
	"circulogo/internal/opencode"
)

// The capability matrix is the thing that makes the two backends
// interchangeable for the UI, so it is asserted rather than assumed. Each
// assertion below corresponds to a real code path — the comment says which —
// because a capability flag that drifts from reality is worse than no flag: the
// UI would offer a control that does nothing.

func TestCapabilities_Opencode(t *testing.T) {
	c := (&opencode.Adapter{}).Capabilities()
	for name, got := range map[string]struct {
		want, got bool
		why       string
	}{
		"permissions": {true, c.Permissions, "ReplyPermissionV2 is a real call"},
		"forms":       {true, c.Forms, "ReplyFormV2 is a real call"},
		"branch":      {true, c.Branch, "pins through an instruction entry"},
		"agents":      {true, c.Agents, "GET /api/agent"},
		"commands":    {true, c.Commands, "POST /api/session/{id}/command"},
		// A prompt while the agent is working is a second turn: the adapter
		// sends it as another prompt, it does not steer. opencode's inbox
		// delivery exists on the wire but is not implemented here, so this
		// must not claim support.
		"steer":      {false, c.Steer, "no inbox delivery in the adapter"},
		"accessMode": {false, c.AccessMode, "SetAccess returns an explicit error"},
	} {
		if got.got != got.want {
			t.Errorf("opencode %s = %v, want %v (%s)", name, got.got, got.want, got.why)
		}
	}
}

func TestCapabilities_Omp(t *testing.T) {
	c := (&omp.Adapter{}).Capabilities()
	for name, got := range map[string]struct {
		want, got bool
		why       string
	}{
		// Approvals resolve inside omp per its own tools.approvalMode; there
		// is no RPC round-trip and ReplyPermission returns an error rather
		// than pretending otherwise.
		"permissions": {false, c.Permissions, "no permission round-trip over RPC"},
		// omp exposes select/confirm/input UI requests, so forms are real.
		"forms":      {true, c.Forms, "select/confirm/input are answered"},
		"steer":      {true, c.Steer, "a prompt while running is sent as steer"},
		"accessMode": {true, c.AccessMode, "AccessSupported is true"},
		// SetBranch is a documented no-op: omp has no instruction channel.
		"branch":   {false, c.Branch, "SetBranch is a no-op"},
		"agents":   {false, c.Agents, "no named-agent registry over RPC"},
		"commands": {true, c.Commands, "command surface exists"},
	} {
		if got.got != got.want {
			t.Errorf("omp %s = %v, want %v (%s)", name, got.got, got.want, got.why)
		}
	}
}

// The two backends must differ where the protocol says they do, and agree
// where it does not. A matrix where they suddenly matched would mean someone
// copied the wrong struct across.
func TestCapabilities_BackendsDifferOnlyWhereTheyShould(t *testing.T) {
	oc := (&opencode.Adapter{}).Capabilities()
	mp := (&omp.Adapter{}).Capabilities()

	differ := map[string][2]bool{
		"permissions": {oc.Permissions, mp.Permissions},
		"steer":       {oc.Steer, mp.Steer},
		"branch":      {oc.Branch, mp.Branch},
		"agents":      {oc.Agents, mp.Agents},
		"accessMode":  {oc.AccessMode, mp.AccessMode},
	}
	agree := map[string][2]bool{
		"forms":    {oc.Forms, mp.Forms},
		"commands": {oc.Commands, mp.Commands},
	}
	for name, v := range differ {
		if v[0] == v[1] {
			t.Errorf("%s should differ between backends, both say %v", name, v[0])
		}
	}
	for name, v := range agree {
		if v[0] != v[1] {
			t.Errorf("%s should be the same for both backends, got %v and %v", name, v[0], v[1])
		}
	}
}

// Capabilities travels on Meta so the composer gets the picker and the
// capability set from one call.
func TestCapabilities_ShipsOnMeta(t *testing.T) {
	raw := `{"agents":[],"models":[],"permissions":false,"branch":false}`
	var m protocol.Meta
	if err := json.Unmarshal([]byte(raw), &m); err != nil {
		t.Fatal(err)
	}
	if m.Capabilities.Permissions {
		t.Error("permissions decoded as true; the flag must default to false")
	}
	if m.Capabilities.AccessMode {
		t.Error("a missing capability key must decode as false, not true")
	}
}
