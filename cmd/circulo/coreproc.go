package main

import (
	"bufio"
	"context"
	"fmt"
	"io"
	"log"
	"net"
	"os"
	"path/filepath"
	"strings"
	"time"

	"github.com/egoist/mygo"

	"github.com/soycanopa/circulo/protocol"
	"github.com/soycanopa/circulo/ui/session"
)

func connect(ctx context.Context, win *mygo.Window, model *session.Model) {
	addr, stop, err := startCore(ctx)
	if err != nil {
		log.Printf("core: %v", err)
		win.Update(func() { model.Fail(err) })
		return
	}
	log.Printf("core: %s", addr)
	defer stop()

	dialer := net.Dialer{Timeout: 5 * time.Second}
	conn, err := dialer.DialContext(ctx, "tcp", addr)
	if err != nil {
		log.Printf("core: dial %s: %v", addr, err)
		win.Update(func() { model.Fail(err) })
		return
	}
	defer conn.Close()

	r := protocol.NewReader(conn)
	for {
		e, err := r.Next()
		if err != nil {
			if ctx.Err() != nil {
				return
			}
			log.Printf("core: read: %v", err)
			win.Update(func() { model.Fail(err) })
			return
		}
		win.Update(func() { _ = model.Apply(e) })
	}
}

// startCore dials CIRCULO_CORE_ADDR when it is set.
// Otherwise it runs cmd/core from the module root and reads the bound address from its stdout.
func startCore(ctx context.Context) (addr string, stop func(), err error) {
	stop = func() {}
	if addr = os.Getenv("CIRCULO_CORE_ADDR"); addr != "" {
		return addr, stop, nil
	}
	root, err := moduleRoot()
	if err != nil {
		return "", stop, err
	}
	cmd := command(ctx, "go", "run", "./cmd/core")
	cmd.Dir = root
	cmd.Stderr = os.Stderr
	stdout, err := cmd.StdoutPipe()
	if err != nil {
		return "", stop, err
	}
	if err := cmd.Start(); err != nil {
		return "", stop, err
	}
	stop = func() {
		if cmd.Cancel != nil {
			_ = cmd.Cancel()
		} else if cmd.Process != nil {
			_ = cmd.Process.Kill()
		}
		_ = cmd.Wait()
	}
	line, err := bufio.NewReader(stdout).ReadString('\n')
	if err != nil {
		stop()
		return "", func() {}, fmt.Errorf("core did not report an address: %w", err)
	}
	go io.Copy(io.Discard, stdout)
	return strings.TrimSpace(line), stop, nil
}

func moduleRoot() (string, error) {
	var starts []string
	if wd, err := os.Getwd(); err == nil {
		starts = append(starts, wd)
	}
	if exe, err := os.Executable(); err == nil {
		starts = append(starts, filepath.Dir(exe))
	}
	for _, start := range starts {
		dir := start
		for {
			if _, err := os.Stat(filepath.Join(dir, "go.mod")); err == nil {
				return dir, nil
			}
			parent := filepath.Dir(dir)
			if parent == dir {
				break
			}
			dir = parent
		}
	}
	return "", fmt.Errorf("set CIRCULO_CORE_ADDR or run from the module that contains go.mod")
}
