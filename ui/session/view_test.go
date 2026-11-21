package session

import (
	"testing"
	"time"

	"github.com/egoist/mygo/ui"
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
