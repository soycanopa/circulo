package opencode

import (
	"context"
	"encoding/json"
	"os"
	"strings"
	"testing"
	"time"

	"circulo/internal/agent/protocol"
)

// TestLiveSmokeAgainstOpencode exercises the adapter against a real
// `opencode serve` binary. It is the drift guard for the v2 wire contract: the
// fixtures in testdata/ freeze the shape we verified against, but they cannot
// tell us that a newer server moved an event, a field, or the prompt body.
// Skipped unless CIRCULOGO_OPENCODE_LIVE=1 (needs an installed, authenticated
// opencode and costs one small model turn):
//
//	CIRCULOGO_OPENCODE_LIVE=1 go test ./internal/opencode/ -run TestLiveSmoke -v
//
// CIRCULOGO_OPENCODE_BIN selects the binary, exactly as main.go does — leave
// it unset to audit whatever the user actually has on PATH, which is the
// default the app launches.
func TestLiveSmokeAgainstOpencode(t *testing.T) {
	if os.Getenv("CIRCULOGO_OPENCODE_LIVE") != "1" {
		t.Skip("set CIRCULOGO_OPENCODE_LIVE=1 to run against the real opencode binary")
	}
	ctx, cancel := context.WithTimeout(context.Background(), 4*time.Minute)
	defer cancel()

	a := NewAdapter(AdapterConfig{
		ProjectID: "live",
		Mode:      ModeManaged,
		Dir:       t.TempDir(),
		Binary:    os.Getenv("CIRCULOGO_OPENCODE_BIN"),
	})
	if err := a.Start(ctx); err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { _ = a.Stop(context.Background()) })

	// Record the version under audit: a green run is only meaningful next to
	// the server it actually ran against.
	info, err := a.client.ServerInfoV2(ctx)
	if err != nil {
		t.Fatalf("server info: %v", err)
	}
	t.Logf("opencode %s (pid %d)", info.Version, info.PID)

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
		case <-ctx.Done():
			t.Fatalf("turn never went idle; last text %q", text.String())
		}
	}
	if !strings.Contains(text.String(), "LIVE-OK") {
		t.Fatalf("expected LIVE-OK in the streamed answer, got %q", text.String())
	}

	// The self-healing invariant (docs/trd.md §3.3): history must hydrate the
	// same text the live stream showed, or reopening a session silently loses
	// the answer. This is the assertion that catches a moved field.
	msgs, err := a.Messages(ctx, sess.ID, 20)
	if err != nil {
		t.Fatal(err)
	}
	var hydrated strings.Builder
	for _, m := range msgs {
		for _, p := range m.Parts {
			if p.Type == protocol.PartText {
				hydrated.WriteString(p.Text)
			}
		}
	}
	if !strings.Contains(hydrated.String(), "LIVE-OK") {
		t.Fatalf("expected LIVE-OK in hydrated history, got %q", hydrated.String())
	}
	if streamed, wet := text.String(), hydrated.String(); !strings.Contains(wet, streamed) &&
		!strings.Contains(streamed, wet) {
		t.Logf("NOTE streamed %q and hydrated %q differ (expected: hydration folds the user turn in)", streamed, wet)
	}

	// The provider surface the composer depends on must still answer.
	meta, err := a.Meta(ctx)
	if err != nil {
		t.Fatalf("meta: %v", err)
	}
	if len(meta.Models) == 0 {
		t.Error("meta returned no models; the composer picker would be empty")
	}

	if err := a.Abort(ctx, sess.ID); err != nil {
		t.Log("abort on idle session returned:", err) // tolerated: nothing to abort
	}
	if err := a.Stop(ctx); err != nil {
		t.Fatal(err)
	}
}
