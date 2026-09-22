package omp

import (
	"context"
	"encoding/json"
	"os"
	"strings"
	"testing"
	"time"

	"circulogo/internal/agent/protocol"
)

// TestLiveSmokeAgainstOmp exercises the adapter against the real `omp`
// binary. It is skipped unless CIRCULOGO_OMP_LIVE=1 (needs an installed,
// authenticated omp and costs a tiny model turn):
//
//	CIRCULOGO_OMP_LIVE=1 go test ./internal/omp/ -run TestLiveSmoke -v
func TestLiveSmokeAgainstOmp(t *testing.T) {
	if os.Getenv("CIRCULOGO_OMP_LIVE") != "1" {
		t.Skip("set CIRCULOGO_OMP_LIVE=1 to run against the real omp binary")
	}
	dir := os.Getenv("CIRCULOGO_OMP_LIVE_DIR")
	if dir == "" {
		dir, _ = os.Getwd()
	}
	a := NewAdapter(AdapterConfig{ProjectID: "live", Dir: dir})
	ctx, cancel := context.WithTimeout(context.Background(), 3*time.Minute)
	defer cancel()
	t.Cleanup(func() { _ = a.Stop(context.Background()) })

	if err := a.Start(ctx); err != nil {
		t.Fatal(err)
	}

	// Picker data arrives from the real catalog.
	meta, err := a.Meta(ctx)
	if err != nil {
		t.Fatal(err)
	}
	if len(meta.Models) == 0 || meta.DefaultModel == "" {
		t.Fatalf("meta incomplete: %d models, default %q", len(meta.Models), meta.DefaultModel)
	}
	t.Logf("catalog: %d models, default %s/%s", len(meta.Models), meta.DefaultProvider, meta.DefaultModel)

	sess, err := a.CreateSession(ctx, "Circulo live smoke")
	if err != nil {
		t.Fatal(err)
	}
	if sess.ID == "" {
		t.Fatal("created session has no id")
	}

	if err := a.Prompt(ctx, sess.ID, protocol.PromptRequest{
		Text: "Reply with exactly: LIVE-OK",
	}); err != nil {
		t.Fatal(err)
	}

	// The turn ends at the first idle status; the answer must have streamed.
	var text strings.Builder
	idle := false
	deadline := time.After(2 * time.Minute)
	for !idle {
		select {
		case env, ok := <-a.Events():
			if !ok {
				t.Fatal("event stream closed before the turn ended")
			}
			switch env.Type {
			case protocol.EventPartUpdated:
				var u protocol.PartUpdated
				if json.Unmarshal(env.Payload, &u) == nil && u.Part.Type == protocol.PartText {
					text.Reset()
					text.WriteString(u.Part.Text)
				}
			case protocol.EventSessionStatus:
				var s protocol.SessionStatus
				if json.Unmarshal(env.Payload, &s) == nil && s.Status == protocol.StatusIdle {
					idle = true
				}
			}
		case <-deadline:
			t.Fatalf("turn never went idle; last text %q", text.String())
		}
	}
	if !strings.Contains(text.String(), "LIVE-OK") {
		t.Fatalf("expected LIVE-OK in the answer, got %q", text.String())
	}

	// History hydrates the turn that just streamed.
	msgs, err := a.Messages(ctx, sess.ID, 10)
	if err != nil {
		t.Fatal(err)
	}
	if len(msgs) < 2 {
		t.Fatalf("hydrated %d messages, want the user turn + answer", len(msgs))
	}

	if err := a.Abort(ctx, sess.ID); err != nil {
		t.Log("abort on idle session returned:", err) // tolerated: nothing to abort
	}
	if err := a.Stop(ctx); err != nil {
		t.Fatal(err)
	}
}
