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
	"github.com/soycanopa/circulo/git"
	"github.com/soycanopa/circulo/persistence"
	"github.com/soycanopa/circulo/protocol"
)

// Registry is the core's project and session state.
// It speaks protocol to UI connections and driver to the provider.
type Registry struct {
	DB      *persistence.Store
	Driver  driver.Driver
	Drivers map[string]driver.Driver
	Boot    error

	mu       sync.Mutex
	seq      int64
	subs     map[chan protocol.Envelope]struct{}
	catalogs map[string][]driver.Model
	chosen   map[string]driver.Model
}

// NewRegistry builds a registry. Driver may be nil when the provider failed to start; Boot explains why.
func NewRegistry(db *persistence.Store, drv driver.Driver, boot error) *Registry {
	return &Registry{
		DB: db, Driver: drv, Boot: boot,
		subs:     map[chan protocol.Envelope]struct{}{},
		catalogs: map[string][]driver.Model{},
		chosen:   map[string]driver.Model{},
	}
}

// Serve accepts loopback clients and watches the provider until ctx is canceled.
func (r *Registry) Serve(ctx context.Context, ln net.Listener) error {
	defer ln.Close()
	for _, d := range r.allDrivers() {
		go d.Watch(ctx, r.onEvent)
	}
	if len(r.allDrivers()) > 0 {
		go r.refresh(ctx)
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
		return r.AddProject(ctx, e.Path, e.Agent)
	case protocol.TypeRewind:
		return r.Rewind(e.Session, e.ID)
	case protocol.TypeCreate:
		return r.NewSession(ctx, e.Project)
	case protocol.TypeOpen:
		return r.Focus(e.ID)
	case protocol.TypePrompt:
		return r.Prompt(ctx, e.ID, e.Text, e.Delivery)
	case protocol.TypeUse:
		return r.UseModel(ctx, e.ID, driver.Model{ID: e.ModelID, ProviderID: e.Provider, Variant: e.Variant})
	case protocol.TypeAbort:
		return r.Abort(ctx, e.ID)
	default:
		return nil
	}
}

// AddProject stores a folder and opens the first session there.
func (r *Registry) AddProject(ctx context.Context, path, agent string) error {
	if agent == "" {
		agent = "opencode"
	}
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
	if r.driverFor(agent) == nil {
		if r.Boot != nil {
			return r.Boot
		}
		return fmt.Errorf("no %s provider", agent)
	}
	id := projectID(abs)
	name := filepath.Base(abs)
	now := time.Now()
	if err := r.DB.UpsertProject(persistence.Project{ID: id, Path: abs, Name: name, Provider: agent, Created: now}); err != nil {
		return err
	}
	stored, err := r.DB.ProjectByPath(abs)
	if err != nil {
		return err
	}
	r.emit(protocol.Envelope{Type: protocol.TypeProject, ID: stored.ID, Path: stored.Path, Name: stored.Name, Agent: stored.Provider})
	return r.NewSession(ctx, stored.ID)
}

// NewSession asks the provider for a session in the project's directory.
func (r *Registry) NewSession(ctx context.Context, projectID string) error {
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
	agent := project.Provider
	if agent == "" {
		agent = "opencode"
	}
	drv := r.driverFor(agent)
	if drv == nil {
		if r.Boot != nil {
			return r.Boot
		}
		return fmt.Errorf("no %s provider", agent)
	}
	sid, err := drv.CreateSession(ctx, project.Path, project.Name)
	if err != nil {
		return err
	}
	sess := persistence.Session{ID: sid, ProjectID: project.ID, Title: project.Name, Provider: agent, Created: time.Now()}
	if err := r.DB.UpsertSession(sess); err != nil {
		return err
	}
	r.emit(protocol.Envelope{Type: protocol.TypeSession, ID: sess.ID, Project: sess.ProjectID, Name: sess.Title, Agent: agent})
	if err := r.ensureCatalog(ctx, project); err != nil {
		r.status(err.Error())
	}
	r.bindModel(ctx, sess.ID, project.Path)
	return r.Focus(sess.ID)
}

// Focus tells the UI to show one session and replays its stored turns.
func (r *Registry) Focus(sessionID string) error {
	sess, err := r.DB.SessionByID(sessionID)
	if err != nil {
		return err
	}
	msgs, err := r.DB.ListMessages(sessionID)
	if err != nil {
		return err
	}
	r.emit(protocol.Envelope{Type: protocol.TypeFocus, ID: sessionID})
	if cp, err := r.DB.LatestCheckpoint(sessionID); err == nil {
		r.emit(protocol.Envelope{Type: protocol.TypeCheckpoint, ID: cp.MessageID, Session: sessionID, Text: cp.Rev})
	}
	for _, m := range msgs {
		r.emit(protocol.Envelope{
			Type: protocol.TypeDelta, ID: m.ID, Session: m.SessionID, Role: m.Role, Text: m.Text,
		})
		if m.Done {
			r.emit(protocol.Envelope{Type: protocol.TypeDone, ID: m.ID, Session: m.SessionID})
		}
	}
	r.emitStoredCatalog(sess.ProjectID)
	return nil
}

// Prompt stores the user turn and asks the provider to run it.
func (r *Registry) Prompt(ctx context.Context, sessionID, text, delivery string) error {
	drv, err := r.sessionDriver(sessionID)
	if err != nil {
		return err
	}
	text = strings.TrimSpace(text)
	if text == "" {
		return errors.New("empty prompt")
	}
	if delivery != "" && delivery != "steer" && delivery != "queue" {
		return fmt.Errorf("unknown delivery %s", delivery)
	}
	id := "usr_" + randomID()
	var rev string
	if delivery == "" {
		if project, err := r.projectOf(sessionID); err == nil {
			if got, err := git.Checkpoint(project.Path); err == nil {
				rev = got
			}
		}
	}
	msg := persistence.Message{ID: id, SessionID: sessionID, Role: "user", Text: text, Done: true}
	if err := r.DB.SaveMessage(msg); err != nil {
		return err
	}
	if rev != "" {
		cp := persistence.Checkpoint{ID: "cp_" + id, SessionID: sessionID, MessageID: id, Rev: rev, Created: time.Now()}
		if err := r.DB.SaveCheckpoint(cp); err != nil {
			return err
		}
		r.emit(protocol.Envelope{Type: protocol.TypeCheckpoint, ID: id, Session: sessionID, Text: rev})
	}
	r.emit(protocol.Envelope{Type: protocol.TypeDelta, ID: id, Session: sessionID, Role: "user", Text: text})
	r.emit(protocol.Envelope{Type: protocol.TypeDone, ID: id, Session: sessionID})
	if delivery == "" {
		r.status("Sending")
	}
	return drv.Prompt(ctx, sessionID, text, delivery)
}

// Rewind restores the worktree to the checkpoint taken before messageID and drops that turn onward.
// The provider session still contains the later turns.
func (r *Registry) Rewind(sessionID, messageID string) error {
	cp, err := r.DB.CheckpointByMessage(messageID)
	if err != nil {
		return err
	}
	if cp.SessionID != sessionID {
		return fmt.Errorf("checkpoint is for %s", cp.SessionID)
	}
	project, err := r.projectOf(sessionID)
	if err != nil {
		return err
	}
	if err := git.Rewind(project.Path, cp.Rev); err != nil {
		return err
	}
	if err := r.DB.DeleteFromMessage(messageID); err != nil {
		return err
	}
	r.status("Rewound the files and the transcript")
	return r.Focus(sessionID)
}

// UseModel switches the model and reasoning effort for one session.
func (r *Registry) UseModel(ctx context.Context, sessionID string, model driver.Model) error {
	drv, err := r.sessionDriver(sessionID)
	if err != nil {
		return err
	}
	if err := drv.SetModel(ctx, sessionID, model); err != nil {
		return err
	}
	r.noteChoice(sessionID, model)
	r.emitChoice(sessionID, model)
	return nil
}

// Abort interrupts the provider turn for one session.
func (r *Registry) Abort(ctx context.Context, sessionID string) error {
	drv, err := r.sessionDriver(sessionID)
	if err != nil {
		return err
	}
	if err := drv.Abort(ctx, sessionID); err != nil {
		return err
	}
	r.status("Idle")
	return nil
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
	case driver.KindModel:
		ref := driver.Model{ID: e.ModelID, ProviderID: e.ProviderID, Variant: e.Variant}
		r.noteChoice(e.SessionID, ref)
		r.emitChoice(e.SessionID, ref)
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
		out = append(out, r.next(protocol.Envelope{Type: protocol.TypeProject, ID: p.ID, Path: p.Path, Name: p.Name, Agent: p.Provider}))
	}
	for _, s := range sessions {
		out = append(out, r.next(protocol.Envelope{Type: protocol.TypeSession, ID: s.ID, Project: s.ProjectID, Name: s.Title, Agent: s.Provider}))
	}
	for id, m := range r.chosen {
		out = append(out, r.next(protocol.Envelope{
			Type: protocol.TypeCurrent, ID: id, Provider: m.ProviderID, ModelID: m.ID, Variant: m.Variant,
		}))
	}
	if len(sessions) > 0 {
		last := sessions[len(sessions)-1]
		out = append(out, r.next(protocol.Envelope{Type: protocol.TypeFocus, ID: last.ID}))
		if cp, err := r.DB.LatestCheckpoint(last.ID); err == nil {
			out = append(out, r.next(protocol.Envelope{Type: protocol.TypeCheckpoint, ID: cp.MessageID, Session: last.ID, Text: cp.Rev}))
		}
		msgs, err := r.DB.ListMessages(last.ID)
		if err == nil {
			for _, m := range msgs {
				out = append(out, r.next(protocol.Envelope{Type: protocol.TypeDelta, ID: m.ID, Session: m.SessionID, Role: m.Role, Text: m.Text}))
				if m.Done {
					out = append(out, r.next(protocol.Envelope{Type: protocol.TypeDone, ID: m.ID, Session: m.SessionID}))
				}
			}
		}
		for _, m := range r.catalogs[last.ProjectID] {
			out = append(out, r.next(protocol.Envelope{
				Type: protocol.TypeModel, ID: m.ID, Provider: m.ProviderID, Name: m.Name,
				Variants: strings.Join(m.Variants, ","),
			}))
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

func (r *Registry) refresh(ctx context.Context) {
	projects, err := r.DB.ListProjects()
	if err != nil {
		r.status(err.Error())
		return
	}
	for _, p := range projects {
		if ctx.Err() != nil {
			return
		}
		if err := r.ensureCatalog(ctx, p); err != nil {
			r.status(err.Error())
		}
	}
	sessions, err := r.DB.ListSessions()
	if err != nil {
		r.status(err.Error())
		return
	}
	paths := map[string]string{}
	for _, p := range projects {
		paths[p.ID] = p.Path
	}
	for _, s := range sessions {
		if ctx.Err() != nil {
			return
		}
		r.bindModel(ctx, s.ID, paths[s.ProjectID])
	}
}

func (r *Registry) ensureCatalog(ctx context.Context, project persistence.Project) error {
	drv := r.driverFor(project.Provider)
	if drv == nil {
		return nil
	}
	r.mu.Lock()
	_, ok := r.catalogs[project.ID]
	r.mu.Unlock()
	if ok {
		return nil
	}
	models, err := drv.Models(ctx, project.Path)
	if err != nil {
		return err
	}
	r.mu.Lock()
	r.catalogs[project.ID] = models
	r.mu.Unlock()
	for _, m := range models {
		r.emit(protocol.Envelope{
			Type: protocol.TypeModel, ID: m.ID, Provider: m.ProviderID, Name: m.Name,
			Variants: strings.Join(m.Variants, ","),
		})
	}
	return nil
}

func (r *Registry) emitStoredCatalog(projectID string) {
	r.mu.Lock()
	models := append([]driver.Model(nil), r.catalogs[projectID]...)
	r.mu.Unlock()
	for _, m := range models {
		r.emit(protocol.Envelope{
			Type: protocol.TypeModel, ID: m.ID, Provider: m.ProviderID, Name: m.Name,
			Variants: strings.Join(m.Variants, ","),
		})
	}
}

func (r *Registry) bindModel(ctx context.Context, sessionID, directory string) {
	drv, err := r.sessionDriver(sessionID)
	if err != nil {
		return
	}
	ref, err := drv.SessionModel(ctx, sessionID)
	if err != nil || ref.ID == "" {
		ref, err = drv.DefaultModel(ctx, directory)
	}
	if err != nil || ref.ID == "" {
		if err != nil {
			r.status(err.Error())
		}
		return
	}
	r.noteChoice(sessionID, ref)
	r.emitChoice(sessionID, ref)
}

func (r *Registry) noteChoice(sessionID string, model driver.Model) {
	r.mu.Lock()
	r.chosen[sessionID] = model
	r.mu.Unlock()
}

func (r *Registry) emitChoice(sessionID string, model driver.Model) {
	r.emit(protocol.Envelope{
		Type: protocol.TypeCurrent, ID: sessionID,
		Provider: model.ProviderID, ModelID: model.ID, Variant: model.Variant,
	})
}

func (r *Registry) driverFor(agent string) driver.Driver {
	if agent == "" {
		agent = "opencode"
	}
	if len(r.Drivers) > 0 {
		return r.Drivers[agent]
	}
	return r.Driver
}

func (r *Registry) allDrivers() []driver.Driver {
	if len(r.Drivers) == 0 {
		if r.Driver == nil {
			return nil
		}
		return []driver.Driver{r.Driver}
	}
	out := make([]driver.Driver, 0, len(r.Drivers))
	for _, d := range r.Drivers {
		if d != nil {
			out = append(out, d)
		}
	}
	return out
}

func (r *Registry) sessionDriver(sessionID string) (driver.Driver, error) {
	sess, err := r.DB.SessionByID(sessionID)
	if err != nil {
		return nil, err
	}
	agent := sess.Provider
	if agent == "" {
		agent = "opencode"
	}
	drv := r.driverFor(agent)
	if drv == nil {
		if r.Boot != nil {
			return nil, r.Boot
		}
		return nil, fmt.Errorf("no %s provider", agent)
	}
	return drv, nil
}

func (r *Registry) projectOf(sessionID string) (persistence.Project, error) {
	sess, err := r.DB.SessionByID(sessionID)
	if err != nil {
		return persistence.Project{}, err
	}
	projects, err := r.DB.ListProjects()
	if err != nil {
		return persistence.Project{}, err
	}
	for _, p := range projects {
		if p.ID == sess.ProjectID {
			return p, nil
		}
	}
	return persistence.Project{}, fmt.Errorf("unknown project %s", sess.ProjectID)
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
