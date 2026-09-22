package omp

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"sync"
	"time"
)

// Frame limits: omp advertises them in the ready frame; these are the
// fallbacks used before/at negotiation (rpc.md: 1 MiB physical, 64 MiB
// reassembled).
const (
	defaultMaxLine        = 1 << 20
	defaultMaxReassembled = 64 << 20
)

// rpcClient speaks the omp JSONL protocol over an already-started child's
// stdio. It correlates responses by request id and hands every non-response
// frame to OnEvent.
type rpcClient struct {
	stdin io.Writer

	mu      sync.Mutex
	nextSeq int

	pendingMu sync.Mutex
	pending   map[string]chan *responseFrame

	// OnEvent receives every stdout frame that is not a command response
	// (session events, UI requests, unknown forward-compatible frames).
	// Called from the read goroutine; must not block.
	OnEvent func(raw json.RawMessage, typ frameType)

	// Ready carries the parsed ready frame; closed after handshake resolves.
	Ready chan struct{}
	ready *readyFrame

	// Done closes when the read loop exits, with its cause.
	Done    chan struct{}
	doneErr error

	writeMu sync.Mutex
}

func newRPCClient(stdin io.Writer) *rpcClient {
	return &rpcClient{
		stdin:   stdin,
		pending: make(map[string]chan *responseFrame),
		Ready:   make(chan struct{}),
		Done:    make(chan struct{}),
	}
}

func (c *rpcClient) setReady(rf *readyFrame) {
	c.ready = rf
	close(c.Ready)
}

// negotiatedMaxReassembled returns the reassembly ceiling in force.
func (c *rpcClient) negotiatedMaxReassembled() int64 {
	if c.ready != nil && c.ready.MaxReassembledFrameBytes > 0 {
		return c.ready.MaxReassembledFrameBytes
	}
	return defaultMaxReassembled
}

// ReadLoop consumes stdout until EOF. Run in its own goroutine.
func (c *rpcClient) ReadLoop(stdout io.Reader) {
	fr := newFrameReader(stdout, defaultMaxLine, defaultMaxReassembled)
	first := true
	for {
		raw, err := fr.Read()
		if err != nil {
			c.doneErr = err
			close(c.Done)
			return
		}
		if first {
			first = false
			rf, err := parseReady(raw)
			if err != nil {
				c.doneErr = err
				close(c.Done)
				return
			}
			c.setReady(rf)
			// Adopt the advertised ceiling instead of the compile-time guess.
			fr.maxReassembled = c.negotiatedMaxReassembled()
			continue
		}
		var env envelope
		if json.Unmarshal(raw, &env) != nil {
			continue
		}
		if env.Type == frameResponse {
			var resp responseFrame
			if json.Unmarshal(raw, &resp) != nil {
				continue
			}
			c.pendingMu.Lock()
			ch := c.pending[resp.ID]
			delete(c.pending, resp.ID)
			c.pendingMu.Unlock()
			if ch != nil {
				ch <- &resp
			}
			continue
		}
		if c.OnEvent != nil {
			c.OnEvent(raw, env.Type)
		}
	}
}

// call writes one command and waits for its correlated response.
func (c *rpcClient) call(ctx context.Context, cmd command, timeout time.Duration) (*responseFrame, error) {
	c.mu.Lock()
	c.nextSeq++
	id := fmt.Sprintf("circulo-%d", c.nextSeq)
	c.mu.Unlock()

	line, err := cmd.marshal(id)
	if err != nil {
		return nil, fmt.Errorf("omp: marshal %s: %w", cmd.payload["type"], err)
	}

	ch := make(chan *responseFrame, 1)
	c.pendingMu.Lock()
	c.pending[id] = ch
	c.pendingMu.Unlock()

	if err := c.writeLine(line); err != nil {
		c.pendingMu.Lock()
		delete(c.pending, id)
		c.pendingMu.Unlock()
		return nil, fmt.Errorf("omp: write %s: %w", cmd.payload["type"], err)
	}

	timer := time.NewTimer(timeout)
	defer timer.Stop()
	select {
	case resp := <-ch:
		return resp, nil
	case <-timer.C:
		c.pendingMu.Lock()
		delete(c.pending, id)
		c.pendingMu.Unlock()
		return nil, fmt.Errorf("omp: %s timed out after %s", cmd.payload["type"], timeout)
	case <-ctx.Done():
		c.pendingMu.Lock()
		delete(c.pending, id)
		c.pendingMu.Unlock()
		return nil, ctx.Err()
	}
}

func (c *rpcClient) writeLine(line []byte) error {
	c.writeMu.Lock()
	defer c.writeMu.Unlock()
	if _, err := c.stdin.Write(append(line, '\n')); err != nil {
		return err
	}
	return nil
}

// send writes a response-less inbound frame (extension_ui_response).
func (c *rpcClient) send(payload map[string]any) error {
	line, err := json.Marshal(payload)
	if err != nil {
		return err
	}
	return c.writeLine(line)
}

// negotiate upgrades to protocol v2 (lossless chunked frames) when the ready
// frame advertises it. Failure leaves us on v1: bounded at the physical frame
// cap, which get_messages pages normally stay under.
func (c *rpcClient) negotiate(ctx context.Context) {
	if !c.ready.supportsV2() {
		return
	}
	_, _ = c.call(ctx, newCommand("negotiate_protocol").with("protocolVersion", 2), 5*time.Second)
}
