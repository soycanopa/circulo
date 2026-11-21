package core

import (
	"context"
	"crypto/sha256"
	"database/sql"
	"encoding/hex"
	"errors"
	"fmt"
	"net"
	"os"
	"path/filepath"
	"strings"
	"sync"
	"time"

	"github.com/soycanopa/circulo/driver"
	"github.com/soycanopa/circulo/persistence"
	"github.com/soycanopa/circulo/protocol"
)

// Registry is the core's project and session state.
// It speaks protocol to UI connections and driver to the provider.
type Registry struct {
	DB     *persistence.Store
	Driver driver.Driver
	Boot   error

	mu   sync.Mutex
	seq  int64
	subs map[chan protocol.Envelope]struct{}
}

// NewRegistry builds a registry. Driver may be nil when the provider failed to start; Boot explains why.
func NewRegistry(db *persistence.Store, drv driver.Driver, boot error) *Registry {
	return &Registry{DB: db, Driver: drv, Boot: boot, subs: map[chan protocol.Envelope]struct{}{}}
}

// Serve accepts loopback clients and watches the provider until ctx is canceled.
func (r *Registry) Serve(ctx context.Context, ln net.Listener) error {
	defer ln.Close()
	if r.Driver != nil {
		go r.Driver.Watch(ctx, r.onEvent)
	}
	go func() {
		<-ctx.Done()
		ln.Close()
	}()
	for {
		conn, err := ln.Accept()
		if err != nil {
			if ctx.Err() != nil {
				return nil
			}
			return err
		}
		go r.serveConn(ctx, conn)
	}
}

func (r *Registry) serveConn(ctx context.Context, conn net.Conn) {
	defer conn.Close()
	ch := make(chan protocol.Envelope, 256)
	writeCtx, cancel := context.WithCancel(ctx)
	defer cancel()
	snap := r.snapshot(ch)
	defer r.unsubscribe(ch)

	for _, e := range snap {
		if err := protocol.Write(conn, e); err != nil {
			return
		}
	}
	go func() {
		for {
			select {
			case <-writeCtx.Done():
				return
			case e := <-ch:
				if err := protocol.Write(conn, e); err != nil {
					return
				}
			}
		}
	}()
	rd := protocol.NewReader(conn)
	for {
		e, err := rd.Next()
		if err != nil {
			return
		}
		if err := r.handle(ctx, e); err != nil {
			r.status(err.Error())
		}
	}
}

func (r *Registry) handle(ctx context.Context, e protocol.Envelope) error {
	if err := e.Validate(); err != nil {
		return err
	}
	switch e.Type {
	case protocol.TypeAdd:
		return r.AddProject(ctx, e.Path)
	case protocol.TypeCreate:
		return r.NewSession(ctx, e.Project)
	case protocol.TypeOpen:
		return r.Focus(e.ID)
	case protocol.TypePrompt:
		return r.Prompt(ctx, e.ID, e.Text)
	default:
		return nil
	}
}

// AddProject stores a folder and opens the first session there.
func (r *Registry) AddProject(ctx context.Context, path string) error {
	abs, err := filepath.Abs(path)
	if err != nil {
		return err
	}
	info, err := os.Stat(abs)
	if err != nil {
		return err
	}
	if !info.IsDir() {
		return fmt.Errorf("%s is not a directory", abs)
	}
	if r.Driver == nil {
		if r.Boot != nil {
			return r.Boot
		}
		return errors.New("no provider")
	}
	id := projectID(abs)
	name := filepath.Base(abs)
	now := time.Now()
	if err := r.DB.UpsertProject(persistence.Project{ID: id, Path: abs, Name: name, Created: now}); err != nil {
		return err
	}
	stored, err := r.DB.ProjectByPath(abs)
	if err != nil {
		return err
	}
	r.emit(protocol.Envelope{Type: protocol.TypeProject, ID: stored.ID, Path: stored.Path, Name: stored.Name})
	return r.NewSession(ctx, stored.ID)
}

// NewSession asks the provider for a session in the project's directory.
func (r *Registry) NewSession(ctx context.Context, projectID string) error {
	if r.Driver == nil {
		if r.Boot != nil {
			return r.Boot
		}
		return errors.New("no provider")
	}
	projects, err := r.DB.ListProjects()
	if err != nil {
		return err
	}
	var project persistence.Project
	for _, p := range projects {
		if p.ID == projectID {
			project = p
			break
		}
	}
	if project.ID == "" {
		return fmt.Errorf("unknown project %s", projectID)
	}
	sid, err := r.Driver.CreateSession(ctx, project.Path, project.Name)
	if err != nil {
		return err
	}
	sess := persistence.Session{ID: sid, ProjectID: project.ID, Title: project.Name, Created: time.Now()}
	if err := r.DB.UpsertSession(sess); err != nil {
		return err
	}
	r.emit(protocol.Envelope{Type: protocol.TypeSession, ID: sess.ID, Project: sess.ProjectID, Name: sess.Title})
	return r.Focus(sess.ID)
}

// Focus tells the UI to show one session and replays its stored turns.
func (r *Registry) Focus(sessionID string) error {
	if _, err := r.DB.SessionByID(sessionID); err != nil {
		return err
	}
	msgs, err := r.DB.ListMessages(sessionID)
	if err != nil {
		return err
	}
	r.emit(protocol.Envelope{Type: protocol.TypeFocus, ID: sessionID})
	for _, m := range msgs {
		r.emit(protocol.Envelope{
			Type: protocol.TypeDelta, ID: m.ID, Session: m.SessionID, Role: m.Role, Text: m.Text,
		})
		if m.Done {
			r.emit(protocol.Envelope{Type: protocol.TypeDone, ID: m.ID, Session: m.SessionID})
		}
	}
	return nil
}

// Prompt stores the user turn and asks the provider to run it.
func (r *Registry) Prompt(ctx context.Context, sessionID, text string) error {
	if r.Driver == nil {
		if r.Boot != nil {
			return r.Boot
		}
		return errors.New("no provider")
	}
	if _, err := r.DB.SessionByID(sessionID); err != nil {
		return err
	}
	text = strings.TrimSpace(text)
	if text == "" {
		return errors.New("empty prompt")
	}
	id := "usr_" + randomID()
	msg := persistence.Message{ID: id, SessionID: sessionID, Role: "user", Text: text, Done: true}
	if err := r.DB.SaveMessage(msg); err != nil {
		return err
	}
	r.emit(protocol.Envelope{Type: protocol.TypeDelta, ID: id, Session: sessionID, Role: "user", Text: text})
	r.emit(protocol.Envelope{Type: protocol.TypeDone, ID: id, Session: sessionID})
	r.status("Sending")
	return r.Driver.Prompt(ctx, sessionID, text)
}

func (r *Registry) onEvent(e driver.Event) {
	if e.SessionID == "" {
		return
	}
	if _, err := r.DB.SessionByID(e.SessionID); err != nil {
		return
	}
	switch e.Kind {
	case driver.KindDelta:
		r.appendDelta(e)
	case driver.KindEnded:
		r.finishText(e)
	case driver.KindDone:
		r.finishTurn(e.SessionID, "")
	case driver.KindFailed:
		if e.Text == "" {
			e.Text = "provider failed"
		}
		r.status(e.Text)
		r.finishTurn(e.SessionID, "")
	}
}

func (r *Registry) appendDelta(e driver.Event) {
	if e.MessageID == "" {
		return
	}
	cur, err := r.DB.Message(e.MessageID)
	if errors.Is(err, sql.ErrNoRows) {
		cur = persistence.Message{ID: e.MessageID, SessionID: e.SessionID, Role: "assistant"}
	} else if err != nil {
		r.status(err.Error())
		return
	}
	cur.Text += e.Text
	cur.Done = false
	if err := r.DB.SaveMessage(cur); err != nil {
		r.status(err.Error())
		return
	}
	r.emit(protocol.Envelope{Type: protocol.TypeDelta, ID: cur.ID, Session: cur.SessionID, Role: cur.Role, Text: e.Text})
	r.status("Streaming")
}

func (r *Registry) finishText(e driver.Event) {
	if e.MessageID == "" {
		r.finishTurn(e.SessionID, "")
		return
	}
	cur, err := r.DB.Message(e.MessageID)
	if errors.Is(err, sql.ErrNoRows) {
		cur = persistence.Message{ID: e.MessageID, SessionID: e.SessionID, Role: "assistant"}
	} else if err != nil {
		r.status(err.Error())
		return
	}
	if e.Text != "" && e.Text != cur.Text {
		rest := e.Text
		if strings.HasPrefix(e.Text, cur.Text) {
			rest = strings.TrimPrefix(e.Text, cur.Text)
		} else {
			cur.Text = ""
		}
		if rest != "" && cur.Text != e.Text {
			cur.Text += rest
			if err := r.DB.SaveMessage(cur); err != nil {
				r.status(err.Error())
				return
			}
			r.emit(protocol.Envelope{Type: protocol.TypeDelta, ID: cur.ID, Session: cur.SessionID, Role: cur.Role, Text: rest})
		}
	}
	cur.Done = true
	cur.Text = firstNonEmpty(e.Text, cur.Text)
	if err := r.DB.SaveMessage(cur); err != nil {
		r.status(err.Error())
		return
	}
	r.emit(protocol.Envelope{Type: protocol.TypeDone, ID: cur.ID, Session: cur.SessionID})
}

func (r *Registry) finishTurn(sessionID, messageID string) {
	msgs, err := r.DB.ListMessages(sessionID)
	if err != nil {
		r.status(err.Error())
		return
	}
	for _, m := range msgs {
		if m.Done {
			continue
		}
		if messageID != "" && m.ID != messageID {
			continue
		}
		m.Done = true
		if err := r.DB.SaveMessage(m); err != nil {
			r.status(err.Error())
			return
		}
		r.emit(protocol.Envelope{Type: protocol.TypeDone, ID: m.ID, Session: m.SessionID})
	}
	r.status("Idle")
}

func (r *Registry) status(text string) {
	r.emit(protocol.Envelope{Type: protocol.TypeStatus, Text: text})
}

func (r *Registry) snapshot(ch chan protocol.Envelope) []protocol.Envelope {
	r.mu.Lock()
	defer r.mu.Unlock()
	r.subs[ch] = struct{}{}
	var out []protocol.Envelope
	out = append(out, r.next(protocol.Envelope{Type: protocol.TypeHello}))
	if r.Boot != nil {
		out = append(out, r.next(protocol.Envelope{Type: protocol.TypeStatus, Text: r.Boot.Error()}))
	}
	projects, err := r.DB.ListProjects()
	if err != nil {
		return append(out, r.next(protocol.Envelope{Type: protocol.TypeStatus, Text: err.Error()}))
	}
	sessions, err := r.DB.ListSessions()
	if err != nil {
		return append(out, r.next(protocol.Envelope{Type: protocol.TypeStatus, Text: err.Error()}))
	}
	for _, p := range projects {
		out = append(out, r.next(protocol.Envelope{Type: protocol.TypeProject, ID: p.ID, Path: p.Path, Name: p.Name}))
	}
	for _, s := range sessions {
		out = append(out, r.next(protocol.Envelope{Type: protocol.TypeSession, ID: s.ID, Project: s.ProjectID, Name: s.Title}))
	}
	if len(sessions) > 0 {
		last := sessions[len(sessions)-1]
		out = append(out, r.next(protocol.Envelope{Type: protocol.TypeFocus, ID: last.ID}))
		msgs, err := r.DB.ListMessages(last.ID)
		if err == nil {
			for _, m := range msgs {
				out = append(out, r.next(protocol.Envelope{Type: protocol.TypeDelta, ID: m.ID, Session: m.SessionID, Role: m.Role, Text: m.Text}))
				if m.Done {
					out = append(out, r.next(protocol.Envelope{Type: protocol.TypeDone, ID: m.ID, Session: m.SessionID}))
				}
			}
		}
	}
	return out
}

func (r *Registry) unsubscribe(ch chan protocol.Envelope) {
	r.mu.Lock()
	delete(r.subs, ch)
	r.mu.Unlock()
}

func (r *Registry) emit(e protocol.Envelope) {
	r.mu.Lock()
	e = r.next(e)
	subs := make([]chan protocol.Envelope, 0, len(r.subs))
	for ch := range r.subs {
		subs = append(subs, ch)
	}
	r.mu.Unlock()
	for _, ch := range subs {
		select {
		case ch <- e:
		default:
		}
	}
}

func (r *Registry) next(e protocol.Envelope) protocol.Envelope {
	r.seq++
	e.V = protocol.Version
	e.Seq = r.seq
	return e
}

func projectID(path string) string {
	sum := sha256.Sum256([]byte(path))
	return "prj_" + hex.EncodeToString(sum[:8])
}

func randomID() string {
	sum := sha256.Sum256([]byte(time.Now().Format(time.RFC3339Nano)))
	return hex.EncodeToString(sum[:6])
}

func firstNonEmpty(a, b string) string {
	if a != "" {
		return a
	}
	return b
}
