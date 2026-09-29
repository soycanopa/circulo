package relay_test

import (
	"bufio"
	"context"
	"encoding/json"
	"net/http"
	"strings"
	"testing"
	"time"

	"circulogo/internal/agent/protocol"
)

// sseFrame is one parsed SSE event: its name ("" for unnamed `data:` only
// frames, which is what the browser delivers to onmessage) and its payload.
type sseFrame struct {
	name string
	data string
}

func collect(t *testing.T, ctx context.Context, url string, until func([]sseFrame) bool) []sseFrame {
	t.Helper()
	req, _ := http.NewRequestWithContext(ctx, http.MethodGet, url, nil)
	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		t.Fatal(err)
	}
	defer resp.Body.Close()
	if ct := resp.Header.Get("Content-Type"); !strings.HasPrefix(ct, "text/event-stream") {
		t.Fatalf("content-type = %q", ct)
	}
	var frames []sseFrame
	done := make(chan struct{})
	go func() {
		defer close(done)
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
					frames = append(frames, cur)
				}
				cur = sseFrame{}
			case strings.HasPrefix(line, ":"):
			case strings.HasPrefix(line, "id: "):
			case strings.HasPrefix(line, "event: "):
				cur.name = strings.TrimPrefix(line, "event: ")
			case strings.HasPrefix(line, "data: "):
				cur.data = strings.TrimPrefix(line, "data: ")
			}
		}
	}()
	deadline := time.After(4 * time.Second)
	for {
		if until(frames) {
			return frames
		}
		select {
		case <-done:
			return frames
		case <-deadline:
			return frames
		case <-time.After(10 * time.Millisecond):
		}
	}
}

func TestSSE_OpensWithReplayFrame(t *testing.T) {
	srv, _ := newServer(t)
	addProject(t, srv, t.TempDir())
	ctx, cancel := context.WithTimeout(context.Background(), 4*time.Second)
	defer cancel()
	frames := collect(t, ctx, srv.URL+"/agent/sse", func(f []sseFrame) bool { return len(f) >= 2 })
	if len(frames) == 0 {
		t.Fatal("no frames received")
	}
	if frames[0].name != "replay" {
		t.Fatalf("first frame name = %q, want %q", frames[0].name, "replay")
	}
	var rf protocol.ReplayFrame
	if err := json.Unmarshal([]byte(frames[0].data), &rf); err != nil {
		t.Fatal(err)
	}
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
	req, _ := http.NewRequestWithContext(ctx, http.MethodGet, srv.URL+"/agent/sse", nil)
	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		t.Fatal(err)
	}
	defer resp.Body.Close()

	// Read until an event frame shows up: the first read only carries the
	// `: connected` comment and the replay control frame.
	var acc strings.Builder
	buf := make([]byte, 4096)
	for !strings.Contains(acc.String(), "\nid: ") {
		n, err := resp.Body.Read(buf)
		if n > 0 {
			acc.Write(buf[:n])
		}
		if err != nil {
			t.Fatalf("stream ended before any event: %v\n%s", err, acc.String())
		}
		if ctx.Err() != nil {
			t.Fatalf("no event frame arrived:\n%s", acc.String())
		}
	}
	body := acc.String()

	i := strings.Index(body, "data: {")
	if i < 0 {
		t.Fatalf("no data frame:\n%s", body)
	}
	rest := body[i+len("data: "):]
	if j := strings.Index(rest, "\n"); j >= 0 {
		rest = rest[:j]
	}
	var env protocol.Envelope
	if err := json.Unmarshal([]byte(rest), &env); err != nil {
		t.Fatalf("event frame is not a valid envelope: %v\n%s", err, rest)
	}
	if env.Seq == 0 {
		t.Error("envelope on the wire has no seq")
	}
	if env.Epoch == "" {
		t.Error("envelope on the wire has no epoch")
	}
}
