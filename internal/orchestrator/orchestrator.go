// Package orchestrator owns the set of configured projects and their adapter
// instances. It fans adapter events out to subscribers (the relay's SSE
// clients) and persists the project list via the store.
//
// Dependency rule: orchestrator may import agent/agent/protocol/store, never
// Wails or relay (AGENTS.md architecture invariants).
package orchestrator

import (
	"context"
	"fmt"
	"os"
	"sort"
	"sync"

	"circulogo/internal/agent"
	"circulogo/internal/agent/protocol"
	"circulogo/internal/store"
	"circulogo/internal/term"
)

// AdapterFactory builds an adapter for a project config. It exists so tests
// can inject scripted adapters instead of the OpenCode one. The composition
// root (main.go) supplies the real factory — orchestrator never imports
// concrete adapters.
type AdapterFactory func(cfg store.Project) (agent.Adapter, error)

// Orchestrator manages project → adapter wiring and event fan-out.
type Orchestrator struct {
	store   *store.Store
	factory AdapterFactory

	mu           sync.RWMutex
	projects     map[string]*project
	order        []string
	terms        *term.Manager
	subs         map[int]*subscriber
	nextSubID    int
	replay       []protocol.Envelope // ring of recent events for late subscribers
	startSeq     []string            // projects pending Start, in settings order
	shutdownOnce sync.Once
}

type project struct {
	cfg     store.Project
	adapter agent.Adapter
	status  protocol.AdapterStatus
	// ready is true only after adapter.Start succeeded. The adapter is
	// registered before Start (so events flow) but its client is nil until
	// then — calling it early crashed the app (panic in the asset server's
	// request goroutine, which has no recover).
	ready bool
}

type subscriber struct {
	ch chan protocol.Envelope
}

// New builds an orchestrator around a store and adapter factory.
func New(st *store.Store, factory AdapterFactory) *Orchestrator {
	return &Orchestrator{
		store:    st,
		factory:  factory,
		projects: map[string]*project{},
		subs:     map[int]*subscriber{},
		terms:    term.NewManager(),
	}
}

// Load restores the persisted projects and starts their adapters. Managed
// adapters start sequentially (port/CPU friendly); a failed adapter keeps the
// project row with an error status (FR-19) instead of aborting the rest.
func (o *Orchestrator) Load(ctx context.Context) error {
	set, err := o.store.Load()
	if err != nil {
		return err
	}
	o.mu.Lock()
	o.startSeq = make([]string, 0, len(set.Projects))
	for _, cfg := range set.Projects {
		o.projects[cfg.ID] = &project{
			cfg: cfg,
			status: protocol.AdapterStatus{
				ProjectID: cfg.ID,
				State:     protocol.AdapterStarting,
			},
		}
		o.order = append(o.order, cfg.ID)
		o.startSeq = append(o.startSeq, cfg.ID)
	}
	o.mu.Unlock()

	for _, id := range o.startSeq {
		o.startAdapter(ctx, id)
	}
	return nil
}

// startAdapter builds and starts one project's adapter in the background.
// It deliberately uses a background context: the caller's ctx (e.g. an HTTP
// request that spawned AddProject) dies when the response is written, while
// the adapter must live until Stop — adapter lifetime is Stop's job, never a
// request's.
func (o *Orchestrator) startAdapter(_ context.Context, id string) {
	go func() {
		o.mu.RLock()
		p := o.projects[id]
		o.mu.RUnlock()
		if p == nil {
			return // removed while starting
		}
		adapter, err := o.factory(p.cfg)
		if err != nil {
			o.setStatus(id, protocol.AdapterError, err.Error())
			return
		}
		o.mu.Lock()
		p.adapter = adapter
		o.mu.Unlock()

		// Pump adapter events into the hub.
		go o.pump(id, adapter.Events())

		if err := adapter.Start(context.Background()); err != nil {
			o.setStatus(id, protocol.AdapterError, err.Error())
			return
		}
		o.mu.Lock()
		p.ready = true
		o.mu.Unlock()
		o.setStatus(id, protocol.AdapterRunning, p.cfg.URL)
	}()
}

// pump forwards adapter events to subscribers until the channel closes.
func (o *Orchestrator) pump(projectID string, events <-chan protocol.Envelope) {
	for env := range events {
		o.broadcast(env)
	}
	o.setStatus(projectID, protocol.AdapterStopped, "")
}

func (o *Orchestrator) setStatus(id, state, detail string) {
	o.mu.Lock()
	p := o.projects[id]
	if p == nil {
		o.mu.Unlock()
		return
	}
	p.status = protocol.AdapterStatus{ProjectID: id, State: state, Detail: detail}
	env, err := protocol.NewEnvelope(protocol.EventAdapterStatus, p.status)
	o.mu.Unlock()
	if err != nil {
		return
	}
	o.broadcast(env)
}

func (o *Orchestrator) broadcast(env protocol.Envelope) {
	o.mu.Lock()
	// Keep a replay ring so subscribers that attach later (or after a UI
	// reload) still see recent state without a full refetch (flow.md §1).
	const replayMax = 1024
	o.replay = append(o.replay, env)
	if len(o.replay) > replayMax {
		o.replay = o.replay[len(o.replay)-replayMax:]
	}
	for _, s := range o.subs {
		select {
		case s.ch <- env:
		default:
			// Slow subscriber: drop rather than block every project's stream.
			// The UI resyncs via reconnect replay + REST refetch.
		}
	}
	o.mu.Unlock()
}

// Subscribe returns a channel of events (replayed from the ring first).
// The returned func unsubscribes and must be called exactly once.
func (o *Orchestrator) Subscribe() (<-chan protocol.Envelope, func()) {
	o.mu.Lock()
	defer o.mu.Unlock()
	id := o.nextSubID
	o.nextSubID++
	ch := make(chan protocol.Envelope, 256)
	// Replay buffered events (copy under lock, send outside would block — the
	// channel is fresh so sends cannot block).
	for _, env := range o.replay {
		ch <- env
	}
	o.subs[id] = &subscriber{ch: ch}
	return ch, func() {
		o.mu.Lock()
		defer o.mu.Unlock()
		delete(o.subs, id)
	}
}

// ProjectView is the REST-facing project descriptor.
type ProjectView struct {
	ID     string `json:"id"`
	Path   string `json:"path"`
	Mode   string `json:"mode"`
	URL    string `json:"url,omitempty"`
	Status string `json:"status"` // starting|running|stopped|error
	Detail string `json:"detail,omitempty"`
}

// AddProject registers and starts a project. Start happens in the background;
// the returned view reports "starting" so the UI can render progress.
func (o *Orchestrator) AddProject(ctx context.Context, path, mode, url string) (ProjectView, error) {
	if mode == "" {
		mode = "managed"
	}
	if mode != "managed" && mode != "attach" {
		return ProjectView{}, fmt.Errorf("orchestrator: unknown mode %q", mode)
	}
	if mode == "managed" {
		info, err := os.Stat(path)
		if err != nil {
			return ProjectView{}, fmt.Errorf("orchestrator: project path: %w", err)
		}
		if !info.IsDir() {
			return ProjectView{}, fmt.Errorf("orchestrator: %s is not a directory", path)
		}
	} else if url == "" {
		return ProjectView{}, fmt.Errorf("orchestrator: attach mode requires url")
	}

	cfg := store.Project{
		ID:   store.ProjectID(path),
		Path: path,
		Mode: mode,
		URL:  url,
	}

	o.mu.Lock()
	if _, exists := o.projects[cfg.ID]; exists {
		o.mu.Unlock()
		return ProjectView{}, fmt.Errorf("orchestrator: project already added: %s", path)
	}
	o.projects[cfg.ID] = &project{
		cfg: cfg,
		status: protocol.AdapterStatus{
			ProjectID: cfg.ID,
			State:     protocol.AdapterStarting,
		},
	}
	o.order = append(o.order, cfg.ID)
	o.mu.Unlock()

	if err := o.persist(); err != nil {
		// Roll back the in-memory row: settings are the source of truth.
		o.mu.Lock()
		delete(o.projects, cfg.ID)
		o.removeOrder(cfg.ID)
		o.mu.Unlock()
		return ProjectView{}, err
	}

	o.startAdapter(ctx, cfg.ID)
	return o.Project(cfg.ID)
}

// RemoveProject stops the adapter (managed → kills opencode serve) and drops
// the row from settings.
func (o *Orchestrator) RemoveProject(_ context.Context, id string) error {
	o.mu.Lock()
	p := o.projects[id]
	if p == nil {
		o.mu.Unlock()
		return fmt.Errorf("orchestrator: unknown project %s", id)
	}
	delete(o.projects, id)
	o.removeOrder(id)
	o.mu.Unlock()

	if p.adapter != nil {
		_ = p.adapter.Stop(context.Background())
	}
	return o.persistLocked()
}

// persistLocked saves settings from current state; caller must NOT hold mu.
func (o *Orchestrator) persist() error { return o.persistLocked() }

func (o *Orchestrator) persistLocked() error {
	o.mu.RLock()
	projects := make([]store.Project, 0, len(o.order))
	for _, id := range o.order {
		if p := o.projects[id]; p != nil {
			projects = append(projects, p.cfg)
		}
	}
	o.mu.RUnlock()
	return o.store.Save(store.Settings{Projects: projects})
}

func (o *Orchestrator) removeOrder(id string) {
	for i, v := range o.order {
		if v == id {
			o.order = append(o.order[:i], o.order[i+1:]...)
			break
		}
	}
}

// Projects lists projects sorted by path for stable output.
func (o *Orchestrator) Projects() []ProjectView {
	o.mu.RLock()
	defer o.mu.RUnlock()
	out := make([]ProjectView, 0, len(o.order))
	for _, id := range o.order {
		p := o.projects[id]
		if p == nil {
			continue
		}
		out = append(out, ProjectView{
			ID:     p.cfg.ID,
			Path:   p.cfg.Path,
			Mode:   p.cfg.Mode,
			URL:    p.cfg.URL,
			Status: p.status.State,
			Detail: p.status.Detail,
		})
	}
	sort.Slice(out, func(i, j int) bool { return out[i].Path < out[j].Path })
	return out
}

// Project returns one project or an error if unknown.
func (o *Orchestrator) Project(id string) (ProjectView, error) {
	o.mu.RLock()
	defer o.mu.RUnlock()
	p := o.projects[id]
	if p == nil {
		return ProjectView{}, fmt.Errorf("orchestrator: unknown project %s", id)
	}
	return ProjectView{
		ID:     p.cfg.ID,
		Path:   p.cfg.Path,
		Mode:   p.cfg.Mode,
		URL:    p.cfg.URL,
		Status: p.status.State,
		Detail: p.status.Detail,
	}, nil
}

// AdapterOf exposes the adapter behind a project for the relay's command
// endpoints. Returns an error for unknown projects.
func (o *Orchestrator) AdapterOf(id string) (agent.Adapter, error) {
	o.mu.RLock()
	defer o.mu.RUnlock()
	p := o.projects[id]
	if p == nil {
		return nil, fmt.Errorf("orchestrator: unknown project %s", id)
	}
	if p.adapter == nil || !p.ready {
		return nil, &NotReadyError{ProjectID: id}
	}
	return p.adapter, nil
}

// NotReadyError means the project's adapter exists but hasn't finished
// starting yet; commands should be retried once adapter.status reports
// running.
type NotReadyError struct{ ProjectID string }

func (e *NotReadyError) Error() string {
	return fmt.Sprintf("orchestrator: project %s adapter is starting", e.ProjectID)
}

// Shutdown stops every adapter (app quit — NFR-4: no orphaned processes).
// Idempotent: both the Wails ServiceShutdown hook and main's defer invoke it,
// whichever path the app exit takes.
// Terms exposes the per-project terminal manager (composer terminal panel).
func (o *Orchestrator) Terms() *term.Manager {
	return o.terms
}

// OpenTerminal spawns an interactive shell rooted at the project directory.
func (o *Orchestrator) OpenTerminal(projectID string) (*term.Terminal, error) {
	o.mu.RLock()
	p := o.projects[projectID]
	o.mu.RUnlock()
	if p == nil {
		return nil, fmt.Errorf("orchestrator: unknown project %s", projectID)
	}
	return o.terms.Open(p.cfg.Path)
}

func (o *Orchestrator) Shutdown() {
	o.shutdownOnce.Do(func() {
		o.terms.CloseAll()
		o.mu.RLock()
		adapters := make([]agent.Adapter, 0, len(o.projects))
		for _, p := range o.projects {
			if p.adapter != nil {
				adapters = append(adapters, p.adapter)
			}
		}
		o.mu.RUnlock()
		for _, a := range adapters {
			_ = a.Stop(context.Background())
		}
	})
}
