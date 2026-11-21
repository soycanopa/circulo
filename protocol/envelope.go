// Package protocol is the only contract the UI process and the core process share.
// Neither side imports the other's packages.
package protocol

import (
	"bufio"
	"encoding/json"
	"fmt"
	"io"
)

const Version = 1

const (
	TypeHello   = "hello"
	TypeDelta   = "part.delta"
	TypeDone    = "turn.done"
	TypeProject = "project.upsert"
	TypeSession = "session.upsert"
	TypeFocus   = "session.focus"
	TypeStatus  = "session.status"
	TypeAdd     = "project.add"
	TypeCreate  = "session.create"
	TypeOpen    = "session.open"
	TypePrompt  = "prompt.send"
)

// Envelope is one versioned event on the loopback connection.
// Text on a part.delta is a chunk appended to message ID, not the whole message.
type Envelope struct {
	V       int    `json:"v"`
	Type    string `json:"type"`
	Seq     int64  `json:"seq"`
	ID      string `json:"id,omitempty"`
	Role    string `json:"role,omitempty"`
	Text    string `json:"text,omitempty"`
	Path    string `json:"path,omitempty"`
	Name    string `json:"name,omitempty"`
	Project string `json:"project,omitempty"`
	Session string `json:"session,omitempty"`
}

func (e Envelope) Validate() error {
	if e.V != Version {
		return fmt.Errorf("protocol: version %d, want %d", e.V, Version)
	}
	if e.Seq < 1 {
		return fmt.Errorf("protocol: seq must start at 1")
	}
	switch e.Type {
	case TypeHello, TypeStatus:
		return nil
	case TypeDelta, TypeDone, TypeFocus, TypeOpen:
		if e.ID == "" {
			return fmt.Errorf("protocol: %s requires id", e.Type)
		}
		return nil
	case TypeProject:
		if e.ID == "" || e.Path == "" {
			return fmt.Errorf("protocol: project.upsert requires id and path")
		}
		return nil
	case TypeSession:
		if e.ID == "" || e.Project == "" {
			return fmt.Errorf("protocol: session.upsert requires id and project")
		}
		return nil
	case TypeAdd:
		if e.Path == "" {
			return fmt.Errorf("protocol: project.add requires path")
		}
		return nil
	case TypeCreate:
		if e.Project == "" {
			return fmt.Errorf("protocol: session.create requires project")
		}
		return nil
	case TypePrompt:
		if e.ID == "" || e.Text == "" {
			return fmt.Errorf("protocol: prompt.send requires id and text")
		}
		return nil
	default:
		return nil
	}
}

// Write encodes one envelope as a single JSON line.
func Write(w io.Writer, e Envelope) error {
	if err := e.Validate(); err != nil {
		return err
	}
	b, err := json.Marshal(e)
	if err != nil {
		return err
	}
	b = append(b, '\n')
	_, err = w.Write(b)
	return err
}

// Reader decodes newline-delimited envelopes.
type Reader struct {
	sc *bufio.Scanner
}

func NewReader(r io.Reader) *Reader {
	sc := bufio.NewScanner(r)
	sc.Buffer(make([]byte, 0, 64*1024), 1<<20)
	return &Reader{sc: sc}
}

func (r *Reader) Next() (Envelope, error) {
	for r.sc.Scan() {
		line := r.sc.Bytes()
		if len(line) == 0 {
			continue
		}
		var e Envelope
		if err := json.Unmarshal(line, &e); err != nil {
			return Envelope{}, fmt.Errorf("protocol: %w", err)
		}
		if err := e.Validate(); err != nil {
			return Envelope{}, err
		}
		return e, nil
	}
	if err := r.sc.Err(); err != nil {
		return Envelope{}, err
	}
	return Envelope{}, io.EOF
}
