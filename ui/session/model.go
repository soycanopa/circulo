// Package session is the UI process's presentation state for one transcript.
// It applies protocol events and never talks to a provider or to the core's packages.
package session

import (
	"fmt"

	"github.com/soycanopa/circulo/protocol"
)

// Message is one turn as the window shows it.
type Message struct {
	ID      string
	Session string
	Role    string
	Text    string
	Done    bool
}

// Project is a folder shown in the sidebar.
type Project struct {
	ID   string
	Path string
	Name string
}

// SessionInfo is one conversation in a project.
type SessionInfo struct {
	ID      string
	Project string
	Title   string
}

// Model is presentation state. Apply it on the UI thread only.
type Model struct {
	Connected bool
	Status    string
	Projects  []Project
	Sessions  []SessionInfo
	Selected  string
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
	case protocol.TypeStatus:
		m.Status = e.Text
	case protocol.TypeProject:
		m.upsertProject(Project{ID: e.ID, Path: e.Path, Name: e.Name})
	case protocol.TypeSession:
		m.upsertSession(SessionInfo{ID: e.ID, Project: e.Project, Title: e.Name})
	case protocol.TypeFocus:
		m.Selected = e.ID
		m.dropSession(e.ID)
	case protocol.TypeDelta:
		m.append(e)
		if e.Role != "user" {
			m.Status = "Streaming"
		}
	case protocol.TypeDone:
		if i, ok := m.lookup(e.ID); ok {
			m.Messages[i].Done = true
		}
		if !m.streaming() && m.Status == "Streaming" {
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
		if m.Messages[i].Session == "" {
			m.Messages[i].Session = e.Session
		}
		return
	}
	if m.index == nil {
		m.index = map[string]int{}
	}
	m.index[e.ID] = len(m.Messages)
	m.Messages = append(m.Messages, Message{ID: e.ID, Session: e.Session, Role: e.Role, Text: e.Text})
}

func (m *Model) upsertProject(p Project) {
	for i := range m.Projects {
		if m.Projects[i].ID == p.ID {
			m.Projects[i] = p
			return
		}
	}
	m.Projects = append(m.Projects, p)
}

func (m *Model) upsertSession(s SessionInfo) {
	for i := range m.Sessions {
		if m.Sessions[i].ID == s.ID {
			m.Sessions[i] = s
			return
		}
	}
	m.Sessions = append(m.Sessions, s)
	if m.Selected == "" {
		m.Selected = s.ID
	}
}

func (m *Model) dropSession(id string) {
	kept := m.Messages[:0]
	next := map[string]int{}
	for _, msg := range m.Messages {
		if msg.Session == id {
			continue
		}
		next[msg.ID] = len(kept)
		kept = append(kept, msg)
	}
	m.Messages = kept
	m.index = next
}

// Visible returns the turns of the selected session.
func (m *Model) Visible() []Message {
	if m.Selected == "" {
		return nil
	}
	var out []Message
	for _, msg := range m.Messages {
		if msg.Session == m.Selected {
			out = append(out, msg)
		}
	}
	return out
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
