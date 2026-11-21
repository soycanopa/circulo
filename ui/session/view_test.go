package session

import (
	"testing"
	"time"

	"github.com/egoist/mygo/ui"

	"github.com/soycanopa/circulo/protocol"
)

func TestViewShowsSettledMessage(t *testing.T) {
	app := NewApp(&Model{
		Status: "Idle",
		Messages: []Message{{
			ID: "h2", Role: "assistant", Done: true,
			Text: "Session alpha is idle.",
		}},
	})
	tt := ui.NewTester(app.View, 980, 680)
	if !tt.HasText("Session alpha is idle.") {
		t.Fatalf("texts %q", tt.Texts())
	}
	if !tt.HasText("Agent") {
		t.Fatalf("texts %q", tt.Texts())
	}
}

func TestViewShowsStreamingText(t *testing.T) {
	app := NewApp(&Model{
		Status: "Streaming",
		Messages: []Message{{
			ID: "live", Role: "assistant",
			Text: "Tokens arrive from the core process.",
		}},
	})
	tt := ui.NewTester(app.View, 980, 680)
	tt.Advance(time.Second)
	if !tt.HasText("Tokens arrive from the core process.") {
		t.Fatalf("texts %q", tt.Texts())
	}
	if !tt.HasText("Streaming") {
		t.Fatalf("texts %q", tt.Texts())
	}
}

func TestSendPrompt(t *testing.T) {
	var got protocol.Envelope
	app := NewApp(&Model{Selected: "ses_1"})
	app.draft = "hello"
	app.Send = func(e protocol.Envelope) { got = e }
	tt := ui.NewTester(app.View, 980, 680)
	if err := tt.Click("Send"); err != nil {
		t.Fatal(err)
	}
	if got.Type != protocol.TypePrompt || got.ID != "ses_1" || got.Text != "hello" || got.Delivery != "" {
		t.Fatalf("%#v", got)
	}
}

func TestSteerAndStop(t *testing.T) {
	var got []protocol.Envelope
	app := NewApp(&Model{Selected: "ses_1", Status: "Streaming"})
	app.draft = "wait"
	app.Send = func(e protocol.Envelope) { got = append(got, e) }
	tt := ui.NewTester(app.View, 980, 680)
	if err := tt.Click("Steer"); err != nil {
		t.Fatal(err)
	}
	app.draft = "next"
	tt = ui.NewTester(app.View, 980, 680)
	if err := tt.Click("Stop"); err != nil {
		t.Fatal(err)
	}
	if len(got) != 2 || got[0].Delivery != "steer" || got[0].Text != "wait" || got[1].Type != protocol.TypeAbort {
		t.Fatalf("%#v", got)
	}
}
