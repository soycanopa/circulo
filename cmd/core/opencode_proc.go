package main

import (
	"bufio"
	"context"
	"fmt"
	"io"
	"net"
	"os"
	"strings"

	"github.com/soycanopa/circulo/driver"
	"github.com/soycanopa/circulo/providers/opencode"
)

// startOpenCode uses OPENCODE_URL when it is set.
// Otherwise it runs `opencode serve` on loopback and reads the URL and password it prints.
func startOpenCode(ctx context.Context) (driver.Driver, func(), error) {
	noop := func() {}
	if base := os.Getenv("OPENCODE_URL"); base != "" {
		return &opencode.Client{
			BaseURL:  strings.TrimRight(base, "/"),
			Password: os.Getenv("OPENCODE_PASSWORD"),
		}, noop, nil
	}
	bin := os.Getenv("OPENCODE_BIN")
	if bin == "" {
		bin = "opencode"
	}
	port, err := freePort()
	if err != nil {
		return nil, noop, err
	}
	cmd := command(ctx, bin, "serve", "--hostname", "127.0.0.1", "--port", port)
	stdout, err := cmd.StdoutPipe()
	if err != nil {
		return nil, noop, err
	}
	cmd.Stderr = os.Stderr
	if err := cmd.Start(); err != nil {
		return nil, noop, fmt.Errorf("start opencode: %w", err)
	}
	stop := func() {
		if cmd.Cancel != nil {
			_ = cmd.Cancel()
		} else if cmd.Process != nil {
			_ = cmd.Process.Kill()
		}
		_ = cmd.Wait()
	}
	base, pass, err := readServerBanner(stdout)
	if err != nil {
		stop()
		return nil, noop, err
	}
	go io.Copy(io.Discard, stdout)
	return &opencode.Client{BaseURL: base, Password: pass}, stop, nil
}

func freePort() (string, error) {
	ln, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		return "", err
	}
	defer ln.Close()
	addr, ok := ln.Addr().(*net.TCPAddr)
	if !ok {
		return "", fmt.Errorf("opencode: unexpected addr %v", ln.Addr())
	}
	return fmt.Sprint(addr.Port), nil
}

func readServerBanner(r io.Reader) (base, password string, err error) {
	sc := bufio.NewScanner(r)
	for sc.Scan() && (base == "" || password == "") {
		line := sc.Text()
		switch {
		case strings.HasPrefix(line, "server listening on "):
			base = strings.TrimPrefix(line, "server listening on ")
		case strings.HasPrefix(line, "server password "):
			password = strings.TrimPrefix(line, "server password ")
		}
	}
	if err := sc.Err(); err != nil {
		return "", "", err
	}
	if base == "" || password == "" {
		return "", "", fmt.Errorf("opencode did not report its address and password")
	}
	return strings.TrimSpace(base), strings.TrimSpace(password), nil
}
