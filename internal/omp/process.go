package omp

import (
	"bytes"
	"fmt"
	"io"
	"os/exec"
	"sync"
	"time"
)

// process owns one spawned `omp --mode rpc` child. The child reads commands
// from stdin and writes frames to stdout; stderr is diagnostic only (kept in a
// small ring for error surfacing, mirroring internal/opencode).
type process struct {
	cmd    *exec.Cmd
	stdin  io.WriteCloser
	stdout io.ReadCloser
	stderr *ringBuffer

	exited chan struct{}
	err    error
	once   sync.Once
}

// startProcess spawns `omp --mode rpc` with cwd = dir. The session root and
// all tool work happen in dir. approvalMode maps the UI access mode onto
// omp's tools.approvalMode (always-ask|write|yolo), applied at start: omp
// exposes no RPC to change it on a live process.
func startProcess(binary, dir, approvalMode string) (*process, error) {
	args := []string{"--mode", "rpc"}
	if approvalMode != "" {
		args = append(args, "--approval-mode", approvalMode)
	}
	return startCommand(binary, args, dir)
}

// startCommand spawns an arbitrary child speaking the omp RPC protocol (the
// test seam: fixtures replay instead of the real binary).
func startCommand(binary string, args []string, dir string) (*process, error) {
	cmd := exec.Command(binary, args...)
	cmd.Dir = dir

	stdin, err := cmd.StdinPipe()
	if err != nil {
		return nil, fmt.Errorf("omp: stdin pipe: %w", err)
	}
	stdout, err := cmd.StdoutPipe()
	if err != nil {
		return nil, fmt.Errorf("omp: stdout pipe: %w", err)
	}
	p := &process{
		cmd:    cmd,
		stdin:  stdin,
		stdout: stdout,
		stderr: newRingBuffer(4 << 10),
		exited: make(chan struct{}),
	}
	cmd.Stderr = p.stderr

	if err := cmd.Start(); err != nil {
		return nil, fmt.Errorf("omp: spawn %s: %w", binary, err)
	}
	go func() {
		p.err = cmd.Wait()
		close(p.exited)
	}()
	return p, nil
}

// errTail returns the tail of the child's stderr — the user-visible cause
// when startup or a session fails (FR-19).
func (p *process) errTail() string { return p.stderr.String() }

// exitedErr reports the child's exit cause, or nil while running.
func (p *process) exitedErr() error {
	select {
	case <-p.exited:
		if p.err == nil {
			return fmt.Errorf("omp exited unexpectedly; output: %s", p.errTail())
		}
		return fmt.Errorf("omp exited: %v; output: %s", p.err, p.errTail())
	default:
		return nil
	}
}

// stop closes stdin (omp's orderly shutdown: drain replies, exit 0), then
// falls back to SIGTERM and SIGKILL. Idempotent.
func (p *process) stop(grace time.Duration) {
	p.once.Do(func() {
		_ = p.stdin.Close()
	})
	select {
	case <-p.exited:
		return
	case <-time.After(grace):
	}
	_ = p.cmd.Process.Kill()
	<-p.exited
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
	if r.drop {
		return "… " + r.buf.String()
	}
	return r.buf.String()
}
