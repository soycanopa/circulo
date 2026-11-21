package core

import (
	"context"
	"net"
	"testing"
	"time"

	"github.com/soycanopa/circulo/protocol"
)

func TestListenRejectsWildcard(t *testing.T) {
	ln, err := Listen("0.0.0.0:0")
	if err == nil {
		ln.Close()
		t.Fatal("wildcard listen succeeded")
	}
}

func TestServeSendsScript(t *testing.T) {
	ln, err := Listen("127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()
	done := make(chan error, 1)
	go func() { done <- Serve(ctx, ln, 0) }()

	conn, err := net.Dial("tcp", ln.Addr().String())
	if err != nil {
		t.Fatal(err)
	}
	defer conn.Close()
	r := protocol.NewReader(conn)
	want := Script()
	for i := range want {
		got, err := r.Next()
		if err != nil {
			t.Fatalf("event %d: %v", i, err)
		}
		if got != want[i] {
			t.Fatalf("event %d\n got %#v\nwant %#v", i, got, want[i])
		}
	}
	conn.Close()
	cancel()
	select {
	case err := <-done:
		if err != nil {
			t.Fatal(err)
		}
	case <-time.After(2 * time.Second):
		t.Fatal("serve did not return")
	}
}
