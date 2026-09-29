package protocol

import (
	"encoding/json"
	"testing"
)

func TestEnvelope_OmitsUnsetResumeFields(t *testing.T) {
	raw, err := json.Marshal(Envelope{Type: EventPartUpdated, Payload: json.RawMessage(`{}`)})
	if err != nil {
		t.Fatal(err)
	}
	// A zero Seq/Epoch must not reach the wire: the client treats a missing
	// seq as "unsequenced" and cannot dedupe on it.
	if got := string(raw); got != `{"type":"part.updated","payload":{}}` {
		t.Errorf("envelope JSON = %s", got)
	}
}

func TestEnvelope_RoundTripsResumeFields(t *testing.T) {
	env := Envelope{Type: EventSessionStatus, Seq: 42, Epoch: "e1", Payload: json.RawMessage(`{"a":1}`)}
	raw, err := json.Marshal(env)
	if err != nil {
		t.Fatal(err)
	}
	var back Envelope
	if err := json.Unmarshal(raw, &back); err != nil {
		t.Fatal(err)
	}
	if back.Seq != 42 || back.Epoch != "e1" || back.Type != EventSessionStatus {
		t.Errorf("round trip lost resume fields: %+v", back)
	}
}

func TestReplayFrame_JSON(t *testing.T) {
	raw, err := json.Marshal(ReplayFrame{Status: ReplayResumed, Epoch: "e9", Seq: 120})
	if err != nil {
		t.Fatal(err)
	}
	if got, want := string(raw), `{"status":"resumed","epoch":"e9","seq":120}`; got != want {
		t.Errorf("replay frame = %s, want %s", got, want)
	}
}
