// Package session is the UI process's presentation state for one transcript.
// It applies protocol events and never talks to a provider or to the core's packages.
package session

import (
	"fmt"
	"strings"

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
	ID    string
	Path  string
	Name  string
	Agent string
}

// SessionInfo is one conversation in a project.
type SessionInfo struct {
	ID      string
	Project string
	Title   string
	Agent   string
}

// Catalog is one model the picker can offer.
type Catalog struct {
	ID       string
	Provider string
	Name     string
	Label    string
	Variants []string
}

// Choice is the model and reasoning effort selected for one session.
type Choice struct {
	Provider string
	ID       string
	Variant  string
}

// Model is presentation state. Apply it on the UI thread only.
type Model struct {
	Connected bool
	Status    string
	Projects  []Project
	Sessions  []SessionInfo
	Selected  string
	Messages  []Message
	Models    []Catalog
	choice    map[string]Choice
	rewind    map[string]string
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
		m.upsertProject(Project{ID: e.ID, Path: e.Path, Name: e.Name, Agent: e.Agent})
	case protocol.TypeSession:
		m.upsertSession(SessionInfo{ID: e.ID, Project: e.Project, Title: e.Name, Agent: e.Agent})
	case protocol.TypeFocus:
		m.Selected = e.ID
		m.dropSession(e.ID)
		m.Models = nil
		delete(m.rewind, e.ID)
	case protocol.TypeCheckpoint:
		if m.rewind == nil {
			m.rewind = map[string]string{}
		}
		m.rewind[e.Session] = e.ID
	case protocol.TypeDelta:
		m.append(e)
		if e.Role != "user" {
			m.Status = "Streaming"
		}
	case protocol.TypeModel:
		m.upsertCatalog(e)
	case protocol.TypeCurrent:
		m.setChoice(e)
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

// RewindID is the message whose checkpoint can restore the selected session.
func (m *Model) RewindID() string {
	if m.rewind == nil {
		return ""
	}
	return m.rewind[m.Selected]
}

// Busy is true while a turn is being sent or streamed.
func (m *Model) Busy() bool {
	return m.Status == "Sending" || m.Status == "Streaming"
}

// Selection is the picker label and reasoning effort for the open session.
func (m *Model) Selection() (label, variant string) {
	if m.choice == nil {
		return "", ""
	}
	ch, ok := m.choice[m.Selected]
	if !ok {
		return "", ""
	}
	for _, it := range m.Models {
		if it.ID == ch.ID && it.Provider == ch.Provider {
			return it.Label, ch.Variant
		}
	}
	if ch.ID == "" {
		return "", ch.Variant
	}
	return ch.Provider + "/" + ch.ID, ch.Variant
}

// Lookup finds a catalog entry by the label the picker shows.
func (m *Model) Lookup(label string) (Catalog, bool) {
	for _, it := range m.Models {
		if it.Label == label {
			return it, true
		}
	}
	return Catalog{}, false
}

func (m *Model) upsertCatalog(e protocol.Envelope) {
	item := Catalog{ID: e.ID, Provider: e.Provider, Name: e.Name}
	if e.Variants != "" {
		item.Variants = strings.Split(e.Variants, ",")
	}
	for i := range m.Models {
		if m.Models[i].ID == item.ID && m.Models[i].Provider == item.Provider {
			m.Models[i].ID = item.ID
			m.Models[i].Provider = item.Provider
			m.Models[i].Name = item.Name
			m.Models[i].Variants = item.Variants
			m.relabel()
			return
		}
	}
	m.Models = append(m.Models, item)
	m.relabel()
}

func (m *Model) relabel() {
	counts := map[string]int{}
	for _, it := range m.Models {
		counts[it.Name]++
	}
	for i := range m.Models {
		it := &m.Models[i]
		switch {
		case it.Name != "" && counts[it.Name] == 1:
			it.Label = it.Name
		case it.Name != "":
			it.Label = it.Provider + "/" + it.Name
		default:
			it.Label = it.Provider + "/" + it.ID
		}
	}
}

func (m *Model) setChoice(e protocol.Envelope) {
	if m.choice == nil {
		m.choice = map[string]Choice{}
	}
	m.choice[e.ID] = Choice{Provider: e.Provider, ID: e.ModelID, Variant: e.Variant}
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
