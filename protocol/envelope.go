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
	TypeHello       = "hello"
	TypeDelta       = "part.delta"
	TypeDone        = "turn.done"
	TypeProject     = "project.upsert"
	TypeSession     = "session.upsert"
	TypeFocus       = "session.focus"
	TypeStatus      = "session.status"
	TypeAdd         = "project.add"
	TypeCreate      = "session.create"
	TypeOpen        = "session.open"
	TypePrompt      = "prompt.send"
	TypeModel       = "model.upsert"
	TypeCurrent     = "model.current"
	TypeUse         = "model.set"
	TypeAbort       = "session.abort"
	TypeCheckpoint  = "checkpoint.upsert"
	TypeRewind      = "checkpoint.rewind"
	TypePermit      = "permission.request"
	TypePermitReply = "permission.reply"
)

// Envelope is one versioned event on the loopback connection.
// Text on a part.delta is a chunk appended to message ID, not the whole message.
type Envelope struct {
	V        int    `json:"v"`
	Type     string `json:"type"`
	Seq      int64  `json:"seq"`
	ID       string `json:"id,omitempty"`
	Role     string `json:"role,omitempty"`
	Text     string `json:"text,omitempty"`
	Path     string `json:"path,omitempty"`
	Name     string `json:"name,omitempty"`
	Project  string `json:"project,omitempty"`
	Session  string `json:"session,omitempty"`
	Provider string `json:"provider,omitempty"`
	ModelID  string `json:"model,omitempty"`
	Variant  string `json:"variant,omitempty"`
	// Variants is a comma-separated list of reasoning-effort ids for one model.
	// On permission.request it is one choice per line: id, label, decision, tab-separated.
	Variants string `json:"variants,omitempty"`
	// Delivery is empty for a normal prompt, or steer or queue while a turn is running.
	Delivery string `json:"delivery,omitempty"`
	// Agent is the provider that owns a project or session: opencode or muse.
	Agent string `json:"agent,omitempty"`
	// Requirement is the approval race guard a permission reply must echo.
	Requirement string `json:"requirement,omitempty"`
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
		if e.Delivery != "" && e.Delivery != "steer" && e.Delivery != "queue" {
			return fmt.Errorf("protocol: prompt.send delivery must be steer or queue")
		}
		return nil
	case TypeModel:
		if e.ID == "" || e.Provider == "" {
			return fmt.Errorf("protocol: model.upsert requires id and provider")
		}
		return nil
	case TypeCurrent, TypeUse:
		if e.ID == "" || e.Provider == "" || e.ModelID == "" {
			return fmt.Errorf("protocol: %s requires id, provider, and model", e.Type)
		}
		return nil
	case TypeAbort:
		if e.ID == "" {
			return fmt.Errorf("protocol: session.abort requires id")
		}
		return nil
	case TypeCheckpoint:
		if e.ID == "" || e.Session == "" || e.Text == "" {
			return fmt.Errorf("protocol: checkpoint.upsert requires id, session, and text")
		}
		return nil
	case TypeRewind:
		if e.ID == "" || e.Session == "" {
			return fmt.Errorf("protocol: checkpoint.rewind requires id and session")
		}
		return nil
	case TypePermit:
		if e.ID == "" || e.Session == "" || e.Name == "" {
			return fmt.Errorf("protocol: permission.request requires id, session, and name")
		}
		return nil
	case TypePermitReply:
		if e.ID == "" || e.Session == "" || e.Text == "" || e.Requirement == "" {
			return fmt.Errorf("protocol: permission.reply requires id, session, text, and requirement")
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
