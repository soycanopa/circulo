package session

import (
	"testing"

	"github.com/soycanopa/circulo/protocol"
)

func TestApplyStreamsThenSettles(t *testing.T) {
	var m Model
	steps := []protocol.Envelope{
		{V: protocol.Version, Type: protocol.TypeHello, Seq: 1},
		{V: protocol.Version, Type: protocol.TypeDelta, Seq: 2, ID: "a", Role: "assistant", Text: "hel"},
		{V: protocol.Version, Type: protocol.TypeDelta, Seq: 3, ID: "a", Role: "assistant", Text: "lo"},
		{V: protocol.Version, Type: "widget.future", Seq: 4},
		{V: protocol.Version, Type: protocol.TypeDone, Seq: 5, ID: "a"},
	}
	for _, e := range steps {
		if err := m.Apply(e); err != nil {
			t.Fatal(err)
		}
	}
	if len(m.Messages) != 1 || m.Messages[0].Text != "hello" || !m.Messages[0].Done {
		t.Fatalf("%#v", m.Messages)
	}
	if m.Status != "Idle" {
		t.Fatalf("status %q", m.Status)
	}
}

func TestFocusReplaysASession(t *testing.T) {
	var m Model
	steps := []protocol.Envelope{
		{V: 1, Type: protocol.TypeSession, Seq: 1, ID: "ses_a", Project: "p", Name: "a"},
		{V: 1, Type: protocol.TypeDelta, Seq: 2, ID: "m1", Session: "ses_a", Role: "user", Text: "old"},
		{V: 1, Type: protocol.TypeFocus, Seq: 3, ID: "ses_a"},
		{V: 1, Type: protocol.TypeDelta, Seq: 4, ID: "m1", Session: "ses_a", Role: "user", Text: "old"},
	}
	for _, e := range steps {
		if err := m.Apply(e); err != nil {
			t.Fatal(err)
		}
	}
	if got := m.Visible(); len(got) != 1 || got[0].Text != "old" {
		t.Fatalf("%#v", got)
	}
}
