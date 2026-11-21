// Package session is the UI process's presentation state for one transcript.
// It applies protocol events and never talks to a provider or to the core's packages.
package session

import (
	"fmt"

	"github.com/soycanopa/circulo/protocol"
)

// Message is one turn as the window shows it.
type Message struct {
	ID   string
	Role string
	Text string
	Done bool
}

// Model is presentation state. Apply it on the UI thread only.
type Model struct {
	Connected bool
	Status    string
	Messages  []Message
	index     map[string]int
}

// Apply folds one envelope into the transcript.
// Unknown event types are ignored so a newer core can speak ahead of this window.
func (m *Model) Apply(e protocol.Envelope) error {
	if err := e.Validate(); err != nil {
		return err
	}
	switch e.Type {
	case protocol.TypeHello:
		m.Connected = true
		if m.Status == "" {
			m.Status = "Core connected"
		}
	case protocol.TypeDelta:
		m.append(e)
		m.Status = "Streaming"
	case protocol.TypeDone:
		if i, ok := m.lookup(e.ID); ok {
			m.Messages[i].Done = true
		}
		if !m.streaming() {
			m.Status = "Idle"
		}
	default:
	}
	return nil
}

// Fail records a connection error for the status line.
func (m *Model) Fail(err error) {
	if err == nil {
		return
	}
	m.Status = err.Error()
}

func (m *Model) append(e protocol.Envelope) {
	if i, ok := m.lookup(e.ID); ok {
		m.Messages[i].Text += e.Text
		if m.Messages[i].Role == "" {
			m.Messages[i].Role = e.Role
		}
		return
	}
	if m.index == nil {
		m.index = map[string]int{}
	}
	m.index[e.ID] = len(m.Messages)
	m.Messages = append(m.Messages, Message{ID: e.ID, Role: e.Role, Text: e.Text})
}

func (m *Model) lookup(id string) (int, bool) {
	if m.index == nil {
		return 0, false
	}
	i, ok := m.index[id]
	return i, ok
}

func (m *Model) streaming() bool {
	for _, msg := range m.Messages {
		if !msg.Done {
			return true
		}
	}
	return false
}

func (m *Model) String() string {
	return fmt.Sprintf("%d messages, %s", len(m.Messages), m.Status)
}
