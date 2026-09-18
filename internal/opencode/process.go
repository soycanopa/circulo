package opencode

import (
	"bytes"
	"context"
	"fmt"
	"net"
	"os/exec"
	"sync"
	"syscall"
	"time"
)

// FreePort asks the kernel for a free TCP port. Small TOCTOU window between
// close and the server binding it — acceptable on loopback; the adapter
// surfaces a bind failure as project error (docs/trd.md §10.5).
func FreePort() (int, error) {
	l, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		return 0, err
	}
	defer l.Close()
	return l.Addr().(*net.TCPAddr).Port, nil
}

// Managed owns one spawned `opencode serve` process.
type Managed struct {
	Binary string
	Dir    string
	Port   int

	cmd    *exec.Cmd
	stderr *ringBuffer
	done   chan error
}

// StartManaged spawns `opencode serve --hostname 127.0.0.1 --port <port>` with
// cwd = dir. The server is rooted at dir, which is how project scoping works
// (docs/trd.md §3.1).
func StartManaged(ctx context.Context, binary, dir string, port int) (*Managed, error) {
	if binary == "" {
		binary = "opencode"
	}
	path, err := exec.LookPath(binary)
	if err != nil {
		return nil, fmt.Errorf("opencode binary %q not found in PATH: %w", binary, err)
	}
	m := &Managed{
		Binary: path,
		Dir:    dir,
		Port:   port,
		stderr: newRingBuffer(4096),
	}
	cmd := exec.CommandContext(ctx, path, "serve", "--hostname", "127.0.0.1", "--port", fmt.Sprint(port))
	cmd.Dir = dir
	cmd.Stdout = m.stderr // serve logs its listening line here; keep as context for errors
	cmd.Stderr = m.stderr
	m.cmd = cmd
	if err := cmd.Start(); err != nil {
		return nil, fmt.Errorf("opencode spawn failed: %w", err)
	}
	m.done = make(chan error, 1)
	go func() { m.done <- cmd.Wait() }()
	return m, nil
}

// ErrTail returns the last bytes of the server's output — the user-visible
// cause when startup or health fails (FR-19).
func (m *Managed) ErrTail() string { return m.stderr.String() }

// Stop terminates the process: SIGTERM, then SIGKILL after grace.
func (m *Managed) Stop(grace time.Duration) {
	if m.cmd == nil || m.cmd.Process == nil {
		return
	}
	_ = m.cmd.Process.Signal(syscall.SIGTERM)
	select {
	case <-m.done:
		return
	case <-time.After(grace):
		_ = m.cmd.Process.Kill()
		<-m.done
	}
}

// WaitHealthy polls GET /global/health until it succeeds or timeout elapses.
// It also fails fast if the process exits while polling (startup crash).
func (m *Managed) WaitHealthy(ctx context.Context, c *Client, timeout time.Duration) error {
	deadline := time.Now().Add(timeout)
	var lastErr error
	for time.Now().Before(deadline) {
		select {
		case err := <-m.done:
			return fmt.Errorf("opencode exited during startup: %v; output: %s", err, m.ErrTail())
		default:
		}
		hctx, cancel := context.WithTimeout(ctx, 750*time.Millisecond)
		_, lastErr = c.Health(hctx)
		cancel()
		if lastErr == nil {
			return nil
		}
		select {
		case <-ctx.Done():
			return ctx.Err()
		case <-time.After(250 * time.Millisecond):
		}
	}
	return fmt.Errorf("opencode health check timed out after %s: %v; output: %s", timeout, lastErr, m.ErrTail())
}

// ringBuffer is a write-only buffer keeping the last N bytes.
type ringBuffer struct {
	mu   sync.Mutex
	buf  bytes.Buffer
	max  int
	drop bool
}

func newRingBuffer(max int) *ringBuffer { return &ringBuffer{max: max} }

func (r *ringBuffer) Write(p []byte) (int, error) {
	r.mu.Lock()
	defer r.mu.Unlock()
	if r.buf.Len()+len(p) > r.max {
		// Compact: keep the tail half, mark that history was dropped.
		keep := r.buf.Bytes()
		if len(keep) > r.max/2 {
			keep = keep[len(keep)-r.max/2:]
		}
		r.buf.Reset()
		r.buf.Write(keep)
		r.drop = true
	}
	r.buf.Write(p)
	return len(p), nil
}

func (r *ringBuffer) String() string {
	r.mu.Lock()
	defer r.mu.Unlock()
	s := r.buf.String()
	if r.drop {
		s = "…(earlier output trimmed)…\n" + s
	}
	return s
}
