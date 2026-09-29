package relay_test

import (
	"context"
	"strconv"
	"testing"
	"time"

	"circulogo/internal/agent/protocol"
)

// Shared SSE helpers (sseFrame, openSSE, untilSSE, eventFrames, replayFrame)
// live in sse_resume_test.go. The cursor-parsing unit tests live in
// cursor_test.go, which is in the internal package because cursorFromRequest
// is unexported.

func TestSSE_OpensWithReplayFrame(t *testing.T) {
	srv, _ := newServer(t)
	addProject(t, srv, t.TempDir())
	ctx, cancel := context.WithTimeout(context.Background(), 4*time.Second)
	defer cancel()
	ch, closeFn := openSSE(t, ctx, srv.URL+"/agent/sse")
	defer closeFn()
	frames := untilSSE(t, ch, func(f []sseFrame) bool { return len(f) >= 2 })
	rf := replayFrame(t, frames)
	if rf.Status != protocol.ReplayFull {
		t.Errorf("status = %s, want %s", rf.Status, protocol.ReplayFull)
	}
	if rf.Epoch == "" {
		t.Error("replay frame must carry the epoch")
	}
}

// Every event frame carries an id: line, which is what lets a native
// EventSource reconnect carry Last-Event-ID without any client code.
func TestSSE_WritesIDPerEvent(t *testing.T) {
	srv, _ := newServer(t)
	addProject(t, srv, t.TempDir())
	ctx, cancel := context.WithTimeout(context.Background(), 4*time.Second)
	defer cancel()
	ch, closeFn := openSSE(t, ctx, srv.URL+"/agent/sse")
	defer closeFn()
	frames := untilSSE(t, ch, func(f []sseFrame) bool { return len(eventFrames(f)) > 0 })
	evs := eventFrames(frames)
	if len(evs) == 0 {
		t.Fatal("no event frames")
	}
	f := evs[0]
	if f.id == "" {
		t.Errorf("event frame has no id: line: %+v", f)
	}
	env := f.envelope(t)
	if env.Seq == 0 {
		t.Error("envelope on the wire has no seq")
	}
	if env.Epoch == "" {
		t.Error("envelope on the wire has no epoch")
	}
	// The id must agree with the payload, or a Last-Event-ID resume is a lie.
	if got, want := f.id, strconv.FormatUint(env.Seq, 10); got != want {
		t.Errorf("id line = %q, want %q", got, want)
	}
}
