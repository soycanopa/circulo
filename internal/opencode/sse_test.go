package opencode

import (
	"io"
	"os"
	"strings"
	"testing"
)

func TestReadFrames_V2FixtureTurn(t *testing.T) {
	raw, err := os.ReadFile("testdata/v2-turn-basic.sse")
	if err != nil {
		t.Fatal(err)
	}
	frames := collect(t, strings.NewReader(string(raw)))
	if len(frames) == 0 {
		t.Fatal("expected frames from fixture, got 0")
	}
	types := make([]string, 0, len(frames))
	for _, f := range frames {
		env, err := parseV2Event(f)
		if err != nil {
			t.Fatalf("decode frame: %v\nframe: %.120s", err, f)
		}
		types = append(types, env.Type)
	}
	// First frame must be server.connected (protocol invariant: first event on
	// a fresh /api/event connection).
	if types[0] != "server.connected" {
		t.Errorf("first event = %q, want server.connected", types[0])
	}
	for _, want := range []string{
		"session.inbox.enqueued", "session.execution.started", "session.text.started",
		"session.text.delta", "session.text.ended", "session.tool.success",
		"session.step.ended", "session.execution.succeeded",
	} {
		found := false
		for _, ty := range types {
			if ty == want {
				found = true
				break
			}
		}
		if !found {
			t.Errorf("fixture missing expected event type %q\nhave: %v", want, types)
		}
	}
}

func TestReadFrames_SkipCommentsAndKeepalives(t *testing.T) {
	// Comments terminated by a blank line (proxy keep-alives) produce no frame.
	stream := ": keep-alive\n\n" +
		"data: {\"id\":\"evt_1\",\"type\":\"server.connected\",\"data\":{}}\n\n" +
		": heartbeat\n\n" +
		"data: {\"id\":\"evt_2\",\"type\":\"server.connected\",\"data\":{}}\n\n"
	frames := collect(t, strings.NewReader(stream))
	if len(frames) != 2 {
		t.Fatalf("got %d frames, want 2: %q", len(frames), frames)
	}
	if !strings.Contains(frames[1].String(), "evt_2") {
		t.Errorf("second frame = %s", frames[1])
	}
}

func TestReadFrames_MultiLineData(t *testing.T) {
	stream := "data: line1\ndata: line2\n\n"
	frames := collect(t, strings.NewReader(stream))
	if len(frames) != 1 {
		t.Fatalf("got %d frames, want 1", len(frames))
	}
	if got := frames[0].String(); got != "line1\nline2" {
		t.Errorf("frame = %q, want %q", got, "line1\nline2")
	}
}

func TestReadFrames_TruncatedTailDropped(t *testing.T) {
	stream := "data: {\"complete\":true}\n\ndata: {\"trunc"
	frames := collect(t, strings.NewReader(stream))
	if len(frames) != 1 {
		t.Fatalf("got %d frames, want 1 (truncated tail must be dropped)", len(frames))
	}
}

// collect runs ReadFrames to completion and returns every frame it emitted.
func collect(t *testing.T, r io.Reader) []Frame {
	t.Helper()
	ch := make(chan Frame, 64)
	done := make(chan error, 1)
	go func() { done <- ReadFrames(r, ch) }()
	var out []Frame
	for {
		select {
		case f := <-ch:
			out = append(out, f)
		case <-done:
			for {
				select {
				case f := <-ch:
					out = append(out, f)
				default:
					return out
				}
			}
		}
	}
}
