package relay_test

import (
	"bufio"
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"strings"
	"testing"
	"time"

	"circulo/internal/agent/protocol"
)

// sseFrame is one parsed SSE frame. name is "" for ordinary event frames,
// which is what the browser delivers to onmessage.
type sseFrame struct {
	name string
	id   string
	data string
}

func (f sseFrame) envelope(t *testing.T) protocol.Envelope {
	t.Helper()
	var env protocol.Envelope
	if err := json.Unmarshal([]byte(f.data), &env); err != nil {
		t.Fatalf("frame is not an envelope: %v (%s)", err, f.data)
	}
	return env
}

// openSSE opens a stream and returns a channel of frames as they arrive.
func openSSE(t *testing.T, ctx context.Context, url string) (<-chan sseFrame, func()) {
	t.Helper()
	req, _ := http.NewRequestWithContext(ctx, http.MethodGet, url, nil)
	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		t.Fatal(err)
	}
	out := make(chan sseFrame, 128)
	go func() {
		defer close(out)
		r := bufio.NewReader(resp.Body)
		var cur sseFrame
		for {
			line, err := r.ReadString('\n')
			if err != nil {
				return
			}
			line = strings.TrimRight(line, "\r\n")
			switch {
			case line == "":
				if cur.data != "" {
					out <- cur
				}
				cur = sseFrame{}
			case strings.HasPrefix(line, ":"):
			case strings.HasPrefix(line, "id: "):
				cur.id = strings.TrimPrefix(line, "id: ")
			case strings.HasPrefix(line, "event: "):
				cur.name = strings.TrimPrefix(line, "event: ")
			case strings.HasPrefix(line, "data: "):
				cur.data = strings.TrimPrefix(line, "data: ")
			}
		}
	}()
	return out, func() { resp.Body.Close() }
}

// untilSSE reads until pred holds, the stream ends, or the deadline passes.
func untilSSE(t *testing.T, ch <-chan sseFrame, pred func([]sseFrame) bool) []sseFrame {
	t.Helper()
	var acc []sseFrame
	deadline := time.After(3 * time.Second)
	for {
		if pred(acc) {
			return acc
		}
		select {
		case f, ok := <-ch:
			if !ok {
				return acc
			}
			acc = append(acc, f)
		case <-deadline:
			return acc
		}
	}
}

func eventFrames(frames []sseFrame) []sseFrame {
	var out []sseFrame
	for _, f := range frames {
		if f.name == "" {
			out = append(out, f)
		}
	}
	return out
}

func replayFrame(t *testing.T, frames []sseFrame) protocol.ReplayFrame {
	t.Helper()
	if len(frames) == 0 {
		t.Fatal("no frames received")
	}
	if frames[0].name != "replay" {
		t.Fatalf("first frame name = %q, want %q", frames[0].name, "replay")
	}
	var rf protocol.ReplayFrame
	if err := json.Unmarshal([]byte(frames[0].data), &rf); err != nil {
		t.Fatalf("replay frame is not decodable: %v", err)
	}
	return rf
}

// adapterFor reaches into the factory the relay tests share, so this file can
// push events on demand.
func adapterFor(t *testing.T, ff *factory, path string) *fakeAdapter {
	t.Helper()
	ff.mu.Lock()
	defer ff.mu.Unlock()
	a, ok := ff.made[path]
	if !ok {
		t.Fatalf("no adapter for %s", path)
	}
	return a
}

// emitWhileDisconnected writes envelopes straight into the adapter's channel,
// which is what the orchestrator's fan-in reads.
func emitWhileDisconnected(t *testing.T, a *fakeAdapter, n int) {
	t.Helper()
	for i := 0; i < n; i++ {
		env, err := protocol.NewEnvelope(protocol.EventSessionStatus, map[string]any{
			"sessionID": fmt.Sprintf("s%d", i),
		})
		if err != nil {
			t.Fatal(err)
		}
		select {
		case a.events <- env:
		case <-time.After(time.Second):
			t.Fatal("adapter event channel full")
		}
	}
}

// TestSSEResume_EndToEnd is the point of the feature: a client that drops
// mid-stream reconnects with its cursor and receives exactly what it missed —
// no refetch, no duplicate, no silence.
func TestSSEResume_EndToEnd(t *testing.T) {
	srv, ff := newServer(t)
	path := t.TempDir()
	addProject(t, srv, path)
	a := adapterFor(t, ff, path)

	ctx, cancel := context.WithCancel(context.Background())
	ch1, close1 := openSSE(t, ctx, srv.URL+"/agent/sse")

	first := untilSSE(t, ch1, func(f []sseFrame) bool { return len(eventFrames(f)) > 0 })
	evs := eventFrames(first)
	if len(evs) == 0 {
		t.Fatal("first connection received no events")
	}
	rf := replayFrame(t, first)
	if rf.Status != protocol.ReplayFull {
		t.Fatalf("first connect status = %s, want %s", rf.Status, protocol.ReplayFull)
	}
	lastSeq := evs[len(evs)-1].envelope(t).Seq
	if lastSeq == 0 {
		t.Fatal("event on the wire has no seq")
	}
	// The id: line must agree with the payload or Last-Event-ID is useless.
	if got, want := evs[len(evs)-1].id, fmt.Sprint(lastSeq); got != want {
		t.Errorf("id line = %q, want %q", got, want)
	}
	close1()
	cancel()

	// Events happen while the client is away.
	emitWhileDisconnected(t, a, 3)
	time.Sleep(200 * time.Millisecond) // let the fan-in drain

	ctx2, cancel2 := context.WithCancel(context.Background())
	defer cancel2()
	url := fmt.Sprintf("%s/agent/sse?epoch=%s&seq=%d", srv.URL, rf.Epoch, lastSeq)
	ch2, close2 := openSSE(t, ctx2, url)
	defer close2()

	got := untilSSE(t, ch2, func(f []sseFrame) bool { return len(eventFrames(f)) >= 3 })
	resumed := replayFrame(t, got)
	if resumed.Status != protocol.ReplayResumed {
		t.Fatalf("status = %s, want %s (the cursor was still inside the ring)", resumed.Status, protocol.ReplayResumed)
	}
	if resumed.Epoch != rf.Epoch {
		t.Errorf("epoch changed under a resume: %q -> %q", rf.Epoch, resumed.Epoch)
	}
	tail := eventFrames(got)
	if len(tail) != 3 {
		t.Fatalf("replayed %d events, want the 3 that were missed", len(tail))
	}
	for _, f := range tail {
		env := f.envelope(t)
		if env.Seq <= lastSeq {
			t.Errorf("resumed stream replayed seq %d, at or before the cursor %d", env.Seq, lastSeq)
		}
		if env.Epoch != rf.Epoch {
			t.Errorf("resumed event has epoch %q, want %q", env.Epoch, rf.Epoch)
		}
	}
}

// A cursor from a previous app run is not resumable, and the server must say
// so rather than serve a ring the client has never seen.
func TestSSEResume_StaleEpochOverHTTP(t *testing.T) {
	srv, _ := newServer(t)
	addProject(t, srv, t.TempDir())
	ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
	defer cancel()
	ch, closeFn := openSSE(t, ctx, srv.URL+"/agent/sse?epoch=previous-run&seq=5")
	defer closeFn()
	frames := untilSSE(t, ch, func(f []sseFrame) bool { return len(f) > 0 })
	rf := replayFrame(t, frames)
	if rf.Status != protocol.ReplayStale {
		t.Errorf("status = %s, want %s", rf.Status, protocol.ReplayStale)
	}
	if rf.Epoch == "previous-run" {
		t.Error("server must report its own epoch, not echo the client's stale one")
	}
}

// Compiles only if the test helpers stay in sync with the shared fakes.
