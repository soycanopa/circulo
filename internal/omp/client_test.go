package omp

import (
	"bufio"
	"context"
	"encoding/json"
	"io"
	"strings"
	"testing"
	"time"
)

const fakeReadyLine = `{"type":"ready","protocolVersion":1,"supportedProtocolVersions":[1,2],"maxFrameBytes":1048576,"maxReassembledFrameBytes":67108864}`

func mustWrite(w io.Writer, s string) {
	if _, err := io.WriteString(w, s); err != nil {
		panic(err)
	}
}

type frameAndRaw struct {
	typ frameType
	raw json.RawMessage
}

// startFakeServer builds an rpcClient over in-memory pipes. One goroutine
// owns the child's stdout, exactly like omp: it writes the ready frame, then
// answers each stdin request via respond ("" = no reply). Frames pushed on
// the returned channel are written to stdout verbatim (unsolicited events).
func startFakeServer(t *testing.T, respond func(cmd map[string]any, id string) string) (*rpcClient, <-chan frameAndRaw, chan<- string) {
	t.Helper()
	stdinR, stdinW := io.Pipe()
	stdoutR, stdoutW := io.Pipe()
	c := newRPCClient(stdinW)
	events := make(chan frameAndRaw, 64)
	c.OnEvent = func(raw json.RawMessage, typ frameType) {
		events <- frameAndRaw{typ: typ, raw: raw}
	}
	go c.ReadLoop(stdoutR)
	push := make(chan string)
	go func() {
		mustWrite(stdoutW, fakeReadyLine+"\n")
		sc := bufio.NewScanner(stdinR)
		sc.Buffer(make([]byte, 64*1024), 4<<20)
		for {
			select {
			case line, ok := <-push:
				if !ok {
					return
				}
				mustWrite(stdoutW, line+"\n")
				continue
			default:
			}
			if !sc.Scan() {
				return
			}
			var cmd map[string]any
			if json.Unmarshal(sc.Bytes(), &cmd) != nil {
				continue
			}
			id, _ := cmd["id"].(string)
			if out := respond(cmd, id); out != "" {
				mustWrite(stdoutW, out+"\n")
			}
		}
	}()
	t.Cleanup(func() {
		stdinR.Close()
		stdinW.Close()
		stdoutW.Close()
	})
	return c, events, push
}

func ackAll(cmd map[string]any, id string) string {
	if id == "" {
		return ""
	}
	return `{"id":"` + id + `","type":"response","command":"` + cmd["type"].(string) + `","success":true}`
}

func TestClientHandshakeNegotiatesV2(t *testing.T) {
	c, _, _ := startFakeServer(t, func(cmd map[string]any, id string) string {
		return `{"id":"` + id + `","type":"response","command":"negotiate_protocol","success":true,"data":{"protocolVersion":2}}`
	})
	select {
	case <-c.Ready:
	case <-time.After(2 * time.Second):
		t.Fatal("ready frame never arrived")
	}
	if !c.ready.supportsV2() {
		t.Fatal("expected v2 advertised")
	}
	c.negotiate(context.Background())
	if c.negotiatedMaxReassembled() != 67108864 {
		t.Fatalf("reassembly ceiling = %d", c.negotiatedMaxReassembled())
	}
}

func TestClientCorrelatesResponsesByID(t *testing.T) {
	c, _, _ := startFakeServer(t, ackAll)
	<-c.Ready

	resp, err := c.call(context.Background(), newCommand("prompt").with("message", "hi"), 2*time.Second)
	if err != nil {
		t.Fatal(err)
	}
	if resp.Command != "prompt" || !resp.Success {
		t.Fatalf("response = %+v", resp)
	}
}

func TestClientSurfacesCommandFailures(t *testing.T) {
	c, _, _ := startFakeServer(t, func(cmd map[string]any, id string) string {
		return `{"id":"` + id + `","type":"response","command":"set_model","success":false,"error":"Model not found: x/y","code":"model_not_found"}`
	})
	<-c.Ready

	resp, err := c.call(context.Background(), newCommand("set_model").with("provider", "x").with("modelId", "y"), 2*time.Second)
	if err != nil {
		t.Fatal(err) // transport-level: correlation worked
	}
	if resp.Success {
		t.Fatal("expected success=false")
	}
	err = resp.ResponseError()
	if !strings.Contains(err.Error(), "model_not_found") || !strings.Contains(err.Error(), "Model not found") {
		t.Fatalf("error should carry message and code, got %q", err)
	}
}

func TestClientDispatchesNonResponseFrames(t *testing.T) {
	c, events, push := startFakeServer(t, ackAll)
	<-c.Ready

	done := make(chan error, 1)
	go func() {
		_, err := c.call(context.Background(), newCommand("prompt"), 2*time.Second)
		done <- err
	}()
	push <- `{"type":"agent_start"}`
	if err := <-done; err != nil {
		t.Fatal(err)
	}
	select {
	case ev := <-events:
		if ev.typ != frameAgentStart {
			t.Fatalf("event = %s", ev.typ)
		}
	case <-time.After(2 * time.Second):
		t.Fatal("event never dispatched")
	}
}

func TestClientCallTimesOutWithoutResponse(t *testing.T) {
	c, _, _ := startFakeServer(t, func(cmd map[string]any, id string) string { return "" })
	<-c.Ready
	if _, err := c.call(context.Background(), newCommand("get_state"), 50*time.Millisecond); err == nil {
		t.Fatal("expected timeout error")
	}
}
