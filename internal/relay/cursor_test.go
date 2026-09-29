package relay

import (
	"net/http"
	"testing"

	"circulogo/internal/agent/protocol"
)

func TestCursorFromRequest_QueryWinsOverHeader(t *testing.T) {
	req, _ := http.NewRequest(http.MethodGet, "/agent/sse?epoch=e7&seq=42", nil)
	req.Header.Set("Last-Event-ID", "9")
	c := cursorFromRequest(req)
	if c.Epoch != "e7" || c.Seq != 42 {
		t.Errorf("cursor = %+v, want {e7 42}", c)
	}
}

func TestCursorFromRequest_FallsBackToLastEventID(t *testing.T) {
	req, _ := http.NewRequest(http.MethodGet, "/agent/sse", nil)
	req.Header.Set("Last-Event-ID", "77")
	c := cursorFromRequest(req)
	if c.Seq != 77 {
		t.Errorf("seq = %d, want 77", c.Seq)
	}
}

func TestCursorFromRequest_RejectsGarbage(t *testing.T) {
	req, _ := http.NewRequest(http.MethodGet, "/agent/sse?epoch=e1&seq=notanumber", nil)
	c := cursorFromRequest(req)
	if c.Seq != 0 {
		t.Errorf("seq = %d, want 0 for an unparsable value", c.Seq)
	}
	if c.Epoch != "e1" {
		t.Errorf("epoch = %q, want it kept even when seq is bad", c.Epoch)
	}
}

func TestCursorFromRequest_EmptyFirstConnect(t *testing.T) {
	req, _ := http.NewRequest(http.MethodGet, "/agent/sse", nil)
	c := cursorFromRequest(req)
	if c != (protocol.Cursor{}) {
		t.Errorf("cursor = %+v, want zero value", c)
	}
}
