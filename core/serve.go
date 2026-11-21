package core

import (
	"context"
	"fmt"
	"io"
	"net"
	"time"

	"github.com/soycanopa/circulo/protocol"
)

// Listen binds a loopback TCP port. An unspecified or non-loopback address is rejected.
func Listen(addr string) (net.Listener, error) {
	if addr == "" {
		addr = "127.0.0.1:0"
	}
	ln, err := net.Listen("tcp", addr)
	if err != nil {
		return nil, err
	}
	host, _, err := net.SplitHostPort(ln.Addr().String())
	if err != nil {
		ln.Close()
		return nil, err
	}
	ip := net.ParseIP(host)
	if ip == nil || !ip.IsLoopback() {
		ln.Close()
		return nil, fmt.Errorf("core listens on loopback only, got %s", host)
	}
	return ln, nil
}

// Serve accepts loopback clients until ctx is canceled.
// delay pauses between chunks of the live reply. History is sent at once.
func Serve(ctx context.Context, ln net.Listener, delay time.Duration) error {
	defer ln.Close()
	go func() {
		<-ctx.Done()
		ln.Close()
	}()
	for {
		conn, err := ln.Accept()
		if err != nil {
			if ctx.Err() != nil {
				return nil
			}
			return err
		}
		go func() {
			defer conn.Close()
			_ = play(ctx, conn, delay)
			<-ctx.Done()
		}()
	}
}

func play(ctx context.Context, w io.Writer, delay time.Duration) error {
	for _, e := range Script() {
		if e.ID == LiveID && e.Type == protocol.TypeDelta && delay > 0 {
			t := time.NewTimer(delay)
			select {
			case <-ctx.Done():
				t.Stop()
				return ctx.Err()
			case <-t.C:
			}
		}
		if err := protocol.Write(w, e); err != nil {
			return err
		}
	}
	return nil
}
