package orchestrator

import (
	"encoding/json"
	"errors"

	"circulogo/internal/agent"
	"testing"
	"time"

	"circulogo/internal/agent/protocol"
	"circulogo/internal/store"
)

func bareOrchestrator(t *testing.T) *Orchestrator {
	t.Helper()
	// The factory is never called here: these tests drive broadcast directly.
	return New(store.New(t.TempDir()+"/settings.json"), func(cfg store.Project) (agent.Adapter, error) {
		return nil, errors.New("not used")
	})
}

func emit(o *Orchestrator, typ string) protocol.Envelope {
	env, err := protocol.NewEnvelope(typ, map[string]any{"projectID": "p1"})
	if err != nil {
		panic(err)
	}
	o.broadcast(env)
	return env
}

func drain(ch <-chan protocol.Envelope) []protocol.Envelope {
	var out []protocol.Envelope
	for {
		select {
		case e, ok := <-ch:
			if !ok {
				return out
			}
			out = append(out, e)
		default:
			return out
		}
	}
}

func TestBroadcast_SequencesFromOne(t *testing.T) {
	o := bareOrchestrator(t)
	// Read the sequence off the wire, not off the caller's local envelope:
	// broadcast stamps the copy it fans out, and that copy is what a client
	// actually sees.
	ch, _, _, cancel := o.Subscribe(protocol.Cursor{})
	defer cancel()
	for i := 0; i < 4; i++ {
		emit(o, protocol.EventSessionStatus)
	}
	got := drain(ch)
	if len(got) != 4 {
		t.Fatalf("received %d envelopes, want 4", len(got))
	}
	for i, env := range got {
		if env.Seq != uint64(i+1) {
			t.Errorf("envelope %d has seq %d, want %d", i, env.Seq, i+1)
		}
	}
}

func TestBroadcast_StampsEpochOnEveryEnvelope(t *testing.T) {
	o := bareOrchestrator(t)
	ch, _, _, cancel := o.Subscribe(protocol.Cursor{})
	defer cancel()
	emit(o, protocol.EventSessionStatus)
	env := drain(ch)
	if len(env) != 1 {
		t.Fatalf("received %d envelopes, want 1", len(env))
	}
	if env[0].Epoch != o.Epoch() {
		t.Fatalf("envelope epoch %q != orchestrator %q", env[0].Epoch, o.Epoch())
	}
}

// The epoch is the whole reason a resume is safe across a restart: the new
// process numbers its events from 1 again, and only the epoch tells the two
// runs apart.
func TestEpoch_DiffersBetweenOrchestrators(t *testing.T) {
	a, b := bareOrchestrator(t), bareOrchestrator(t)
	if a.Epoch() == b.Epoch() {
		t.Fatalf("two processes share epoch %q; a resumed cursor would be trusted across a restart", a.Epoch())
	}
	ca, _, _, cancelA := a.Subscribe(protocol.Cursor{})
	defer cancelA()
	cb, _, _, cancelB := b.Subscribe(protocol.Cursor{})
	defer cancelB()
	emit(a, protocol.EventSessionStatus)
	emit(b, protocol.EventSessionStatus)
	ea, eb := drain(ca), drain(cb)
	if len(ea) != 1 || len(eb) != 1 {
		t.Fatalf("got %d and %d envelopes, want 1 each", len(ea), len(eb))
	}
	// Same sequence number, different epoch: that pair is what makes a
	// restart distinguishable from a continuation.
	if ea[0].Seq != eb[0].Seq {
		t.Fatalf("both processes should start their sequence at 1, got %d and %d", ea[0].Seq, eb[0].Seq)
	}
	if ea[0].Epoch == eb[0].Epoch {
		t.Fatal("epochs must differ")
	}
}

func TestSubscribe_FirstConnectReplaysWholeRing(t *testing.T) {
	o := bareOrchestrator(t)
	emit(o, protocol.EventAdapterStatus)
	emit(o, protocol.EventSessionStatus)
	emit(o, protocol.EventPartUpdated)

	ch, status, seq, cancel := o.Subscribe(protocol.Cursor{})
	defer cancel()
	if status != protocol.ReplayFull {
		t.Errorf("status = %s, want %s", status, protocol.ReplayFull)
	}
	if seq != 3 {
		t.Errorf("replayed seq = %d, want 3", seq)
	}
	if got := len(drain(ch)); got != 3 {
		t.Errorf("replayed %d envelopes, want 3", got)
	}
}

func TestSubscribe_ResumesFromCursor(t *testing.T) {
	o := bareOrchestrator(t)
	first, status0, _, cancel0 := o.Subscribe(protocol.Cursor{})
	_ = status0
	emit(o, protocol.EventAdapterStatus)
	emit(o, protocol.EventSessionStatus)
	// The subscriber saw the first two before the ring filled past it.
	seen := drain(first)
	cancel0()
	if len(seen) != 2 {
		t.Fatalf("live subscriber saw %d, want 2", len(seen))
	}

	emit(o, protocol.EventPartUpdated)
	emit(o, protocol.EventPartUpdated)

	ch, status, _, cancel := o.Subscribe(protocol.Cursor{Epoch: o.Epoch(), Seq: seen[len(seen)-1].Seq})
	defer cancel()
	if status != protocol.ReplayResumed {
		t.Fatalf("status = %s, want %s", status, protocol.ReplayResumed)
	}
	tail := drain(ch)
	if len(tail) != 2 {
		t.Fatalf("replayed %d, want only the 2 missed", len(tail))
	}
	for _, e := range tail {
		if e.Seq <= seen[len(seen)-1].Seq {
			t.Fatalf("replayed seq %d at or before cursor %d", e.Seq, seen[len(seen)-1].Seq)
		}
	}
}

func TestSubscribe_StaleEpochAsksForRefetch(t *testing.T) {
	o := bareOrchestrator(t)
	emit(o, protocol.EventSessionStatus)
	// A cursor from a previous process: its ring is not ours.
	ch, status, _, cancel := o.Subscribe(protocol.Cursor{Epoch: "some-previous-run", Seq: 1})
	defer cancel()
	if status != protocol.ReplayStale {
		t.Errorf("status = %s, want %s", status, protocol.ReplayStale)
	}
	if got := drain(ch); len(got) != 0 {
		t.Errorf("stale cursor replayed %d envelopes, want 0", len(got))
	}
}

// A cursor from the future is impossible in this epoch; trusting it would
// silently strand the client with no events at all.
func TestSubscribe_ImpossibleCursorTreatedAsStale(t *testing.T) {
	o := bareOrchestrator(t)
	emit(o, protocol.EventSessionStatus)
	_, status, _, cancel := o.Subscribe(protocol.Cursor{Epoch: o.Epoch(), Seq: 999})
	defer cancel()
	if status != protocol.ReplayStale {
		t.Errorf("status = %s, want %s", status, protocol.ReplayStale)
	}
}

func TestSubscribe_CursorOlderThanRingReportsGap(t *testing.T) {
	o := bareOrchestrator(t)
	// Overflow the ring so its oldest retained sequence is well past 1.
	for i := 0; i < replayMax+50; i++ {
		emit(o, protocol.EventSessionStatus)
	}
	_, status, _, cancel := o.Subscribe(protocol.Cursor{Epoch: o.Epoch(), Seq: 1})
	defer cancel()
	if status != protocol.ReplayGap {
		t.Errorf("status = %s, want %s", status, protocol.ReplayGap)
	}
}

// A subscriber that stops reading must not cost it events silently: the
// channel is closed so the relay ends the response and the client reconnects
// with a cursor that recovers the exact tail.
func TestBroadcast_ClosesLaggedSubscriber(t *testing.T) {
	o := bareOrchestrator(t)
	ch, _, _, cancel := o.Subscribe(protocol.Cursor{})
	defer cancel()
	// One more than the buffer holds, so the last send overflows.
	for i := 0; i < subBuffer+1; i++ {
		emit(o, protocol.EventSessionStatus)
	}
	deadline := time.After(2 * time.Second)
	for {
		if _, ok := <-ch; !ok {
			return // closed as expected
		}
		select {
		case <-deadline:
			t.Fatal("lagged subscriber channel was never closed")
		default:
		}
	}
}

func TestSubscribe_UnsubscribeIsIdempotentWithLagClose(t *testing.T) {
	o := bareOrchestrator(t)
	ch, _, _, cancel := o.Subscribe(protocol.Cursor{})
	for i := 0; i < subBuffer+1; i++ {
		emit(o, protocol.EventSessionStatus)
	}
	// The lag path already removed and closed it; calling cancel must not
	// panic on a second close.
	cancel()
	drain(ch)
}

func TestBroadcast_PayloadUnaffectedByResumeFields(t *testing.T) {
	o := bareOrchestrator(t)
	env := emit(o, protocol.EventSessionStatus)
	var payload struct {
		ProjectID string `json:"projectID"`
	}
	if err := json.Unmarshal(env.Payload, &payload); err != nil {
		t.Fatal(err)
	}
	if payload.ProjectID != "p1" {
		t.Errorf("payload corrupted by sequencing: %+v", payload)
	}
}
