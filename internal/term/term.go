// Package term runs interactive shell terminals (PTY) per project for the
// composer-adjacent terminal panel. Output fans out to subscribers via
// buffered channels — slow subscribers drop chunks rather than block the
// shell, mirroring the orchestrator's pump policy.
package term

import (
	"crypto/rand"
	"encoding/hex"
	"fmt"
	"os"
	"os/exec"
	"sync"

	"github.com/creack/pty"
)

const (
	// outBuf bounds the subscriber channels; a full channel drops chunks.
	outBuf = 256
)

type Terminal struct {
	ID  string
	dir string

	cmd *exec.Cmd
	tty *os.File

	mu      sync.Mutex
	subs    map[int]chan []byte
	nextSub int
	closed  bool
}

type Manager struct {
	mu    sync.Mutex
	terms map[string]*Terminal
	next  int
}

func NewManager() *Manager {
	return &Manager{terms: map[string]*Terminal{}}
}

func randomID() string {
	b := make([]byte, 8)
	_, _ = rand.Read(b)
	return hex.EncodeToString(b)
}

// Open spawns an interactive shell rooted at dir and starts pumping its
// output to subscribers. The shell is the user's $SHELL (fallback /bin/zsh).
func (m *Manager) Open(dir string) (*Terminal, error) {
	shell := os.Getenv("SHELL")
	if shell == "" {
		shell = "/bin/zsh"
	}
	cmd := exec.Command(shell, "-i")
	cmd.Dir = dir
	cmd.Env = append(os.Environ(), "TERM=xterm-256color")

	tty, err := pty.Start(cmd)
	if err != nil {
		return nil, fmt.Errorf("term: pty start: %w", err)
	}

	t := &Terminal{
		ID:   "term_" + randomID(),
		dir:  dir,
		cmd:  cmd,
		tty:  tty,
		subs: map[int]chan []byte{},
	}

	m.mu.Lock()
	m.next++
	m.terms[t.ID] = t
	m.mu.Unlock()

	go t.pump()
	go t.wait()
	return t, nil
}

func (m *Manager) Get(id string) (*Terminal, error) {
	m.mu.Lock()
	defer m.mu.Unlock()
	t, ok := m.terms[id]
	if !ok {
		return nil, fmt.Errorf("term: unknown terminal %s", id)
	}
	return t, nil
}

func (m *Manager) Close(id string) error {
	t, err := m.Get(id)
	if err != nil {
		return err
	}
	t.close()
	m.mu.Lock()
	delete(m.terms, id)
	m.mu.Unlock()
	return nil
}

// CloseAll tears every terminal down (app shutdown).
func (m *Manager) CloseAll() {
	m.mu.Lock()
	ids := make([]string, 0, len(m.terms))
	for id := range m.terms {
		ids = append(ids, id)
	}
	m.mu.Unlock()
	for _, id := range ids {
		_ = m.Close(id)
	}
}

func (t *Terminal) close() {
	t.mu.Lock()
	if t.closed {
		t.mu.Unlock()
		return
	}
	t.closed = true
	t.mu.Unlock()
	_ = t.tty.Close()
	_ = t.cmd.Process.Kill()
}

// pump reads the PTY master and fans chunks out to subscribers until the
// terminal closes.
func (t *Terminal) pump() {
	buf := make([]byte, 8192)
	for {
		n, err := t.tty.Read(buf)
		if n > 0 {
			chunk := make([]byte, n)
			copy(chunk, buf[:n])
			t.mu.Lock()
			for _, ch := range t.subs {
				select {
				case ch <- chunk:
				default: // slow subscriber: drop the chunk
				}
			}
			t.mu.Unlock()
		}
		if err != nil {
			t.close()
			// let subscribers know the shell exited
			t.mu.Lock()
			for _, ch := range t.subs {
				select {
				case ch <- nil:
				default:
				}
			}
			t.mu.Unlock()
			return
		}
	}
}

func (t *Terminal) wait() {
	_, _ = t.cmd.Process.Wait()
	t.close()
}

// Subscribe registers an output channel; the returned func unsubscribes.
func (t *Terminal) Subscribe() (<-chan []byte, func()) {
	t.mu.Lock()
	defer t.mu.Unlock()
	ch := make(chan []byte, outBuf)
	id := t.nextSub
	t.nextSub++
	t.subs[id] = ch
	return ch, func() {
		t.mu.Lock()
		delete(t.subs, id)
		t.mu.Unlock()
	}
}

// IsClosed reports whether the shell has exited.
func (t *Terminal) IsClosed() bool {
	t.mu.Lock()
	defer t.mu.Unlock()
	return t.closed
}

// Write sends keystrokes to the shell.
func (t *Terminal) Write(data []byte) error {
	t.mu.Lock()
	closed := t.closed
	t.mu.Unlock()
	if closed {
		return fmt.Errorf("term: terminal %s closed", t.ID)
	}
	_, err := t.tty.Write(data)
	return err
}

// Resize sets the PTY window size.
func (t *Terminal) Resize(cols, rows uint16) error {
	t.mu.Lock()
	closed := t.closed
	t.mu.Unlock()
	if closed {
		return fmt.Errorf("term: terminal %s closed", t.ID)
	}
	return pty.Setsize(t.tty, &pty.Winsize{Cols: cols, Rows: rows})
}
