// Package orchestrator owns the set of configured projects and their adapter
// instances. It fans adapter events out to subscribers (the relay's SSE
// clients) and persists the project list via the store.
//
// Dependency rule: orchestrator may import agent/agent/protocol/store, never
// Wails or relay (AGENTS.md architecture invariants).
package orchestrator

import (
	"context"
	"crypto/rand"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"os"
	"sort"
	"strconv"
	"sync"
	"time"

	"circulo/internal/agent"
	"circulo/internal/agent/protocol"
	"circulo/internal/checkpoint"
	"circulo/internal/store"
	"circulo/internal/term"
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
	seq          uint64              // last assigned envelope sequence
	epoch        string              // changes on every process start
	startSeq     []string            // projects pending Start, in settings order
	shutdownOnce sync.Once
	// turns maps session id to the turn currently open, and nextTurn to the
	// last number used for it. Both are the turn-boundary bookkeeping that
	// decides which checkpoint ref an event writes.
	turns    map[string]int
	nextTurn map[string]int
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

const (
	// replayMax bounds the ring of recent events kept for subscribers that
	// attach late or resume. A cursor older than the ring cannot be served
	// and the client is told to refetch instead.
	replayMax = 1024
	// subBuffer is how far a subscriber may lag before its channel is closed.
	// On a full buffer the orchestrator drops the subscriber rather than
	// silently discarding events: closing makes the relay end the response,
	// the client reconnects with its cursor, and the ring delivers exactly
	// what was missed. Silently dropping is what makes a UI strand itself.
	subBuffer = 256
)

// newEpoch returns a value that differs between processes. Sequence numbers
// restart at 1 on every start, so the epoch is the only thing that stops a
// client from treating a fresh run's event 1 as the continuation of an old
// run's event 1. Random rather than a timestamp so two processes started in
// the same millisecond still differ.
func newEpoch() string {
	var b [8]byte
	if _, err := rand.Read(b[:]); err != nil {
		// crypto/rand failing is not recoverable in any useful way here, and
		// a predictable epoch would be worse than a distinctive-but-fixed
		// one; fall back to the clock, which is still unique per process.
		return strconv.FormatInt(time.Now().UnixNano(), 36)
	}
	return hex.EncodeToString(b[:])
}

// New builds an orchestrator around a store and adapter factory.
func New(st *store.Store, factory AdapterFactory) *Orchestrator {
	return &Orchestrator{
		store:    st,
		factory:  factory,
		projects: map[string]*project{},
		subs:     map[int]*subscriber{},
		terms:    term.NewManager(),
		epoch:    newEpoch(),
		turns:    map[string]int{},
		nextTurn: map[string]int{},
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
		o.observeTurnBoundary(projectID, env)
		o.broadcast(env)
	}
	o.setStatus(projectID, protocol.AdapterStopped, "")
}

// keepTurns bounds how many turns of checkpoints a session retains. Refs are
// cheap but not free, and nobody rewinds further back than a few turns.
const keepTurns = 20

// observeTurnBoundary records the workspace state at the edges of a turn.
//
// It keys off session.status rather than the prompt call because the relay
// hands prompts straight to the adapter — the orchestrator never sees them —
// while status is the one event both backends already translate, and means the
// same thing in each. A turn is a busy→idle cycle.
//
// Checkpointing is best-effort and never blocks the event pump: a project that
// is not a git repository simply has no checkpoints, and that is a normal
// state, not a failure to report on every turn.
func (o *Orchestrator) observeTurnBoundary(projectID string, env protocol.Envelope) {
	if env.Type != protocol.EventSessionStatus {
		return
	}
	var st protocol.SessionStatus
	if err := json.Unmarshal(env.Payload, &st); err != nil {
		return
	}
	// The adapter stamps its own project id; trust the pump's, which is the
	// one this orchestrator registered the adapter under.
	o.mu.Lock()
	dir := ""
	if p := o.projects[projectID]; p != nil {
		dir = p.cfg.Path
	}
	if dir == "" {
		o.mu.Unlock()
		return
	}
	key := st.SessionID
	switch st.Status {
	case protocol.StatusBusy:
		if _, open := o.turns[key]; open {
			// A retry re-reports busy; the turn is already open.
			o.mu.Unlock()
			return
		}
		o.nextTurn[key]++
		o.turns[key] = o.nextTurn[key]
		turn := o.turns[key]
		o.mu.Unlock()
		o.capture(dir, key, turn, checkpoint.KindStart)
	case protocol.StatusIdle:
		turn, open := o.turns[key]
		if !open {
			o.mu.Unlock()
			return
		}
		delete(o.turns, key)
		o.mu.Unlock()
		o.capture(dir, key, turn, checkpoint.KindEnd)
		// Pruning is housekeeping; a failure only costs disk until the next
		// turn tries again, so it is not worth a per-turn error path.
		_ = checkpoint.Prune(context.Background(), dir, key, keepTurns)
	default:
		o.mu.Unlock()
	}
}

// capture writes one checkpoint ref. Failures are swallowed on purpose: this
// runs inside the event pump on every turn, and a project that is not a git
// repository is a normal state rather than an error to surface there. Whether
// checkpoints are available is a queryable property (Checkpoints below), so the
// UI asks instead of the pump reporting.
func (o *Orchestrator) capture(dir, sessionID string, turn int, kind checkpoint.Kind) {
	if !checkpoint.Available(dir) {
		return
	}
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()
	_, _ = checkpoint.Capture(ctx, dir, sessionID, turn, kind)
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
	o.seq++
	env.Seq = o.seq
	env.Epoch = o.epoch
	// Keep a replay ring so subscribers that attach later (or resume) still
	// see recent state without a full refetch (flow.md §1).
	o.replay = append(o.replay, env)
	if len(o.replay) > replayMax {
		o.replay = o.replay[len(o.replay)-replayMax:]
	}
	for id, s := range o.subs {
		select {
		case s.ch <- env:
		default:
			// Slow subscriber: drop the subscription instead of the event.
			// Closing the channel ends the relay's response; the client
			// reconnects with its cursor and the ring hands it the exact tail
			// it missed. Dropping the event silently is what strands a UI —
			// the client has no way to learn that something went missing.
			delete(o.subs, id)
			close(s.ch)
		}
	}
	o.mu.Unlock()
}

// Subscribe returns a channel of events plus the replay status that describes
// what the caller received, so the relay can tell the client whether it is
// current (ReplayResumed) or owes it a refetch. The returned func unsubscribes
// and must be called exactly once.
func (o *Orchestrator) Subscribe(after protocol.Cursor) (<-chan protocol.Envelope, protocol.ReplayStatus, uint64, func()) {
	o.mu.Lock()
	defer o.mu.Unlock()
	id := o.nextSubID
	o.nextSubID++
	ch := make(chan protocol.Envelope, subBuffer)

	status := protocol.ReplayFull
	// Replay buffered events (copy under lock, send outside would block — the
	// channel is fresh so sends cannot block).
	switch {
	case after.Epoch == "" || (after.Epoch == o.epoch && after.Seq == 0):
		// First connection: hand over the whole ring, best effort.
		for _, env := range o.replay {
			ch <- env
		}
	case after.Epoch != o.epoch || after.Seq > o.seq:
		// Either the app restarted between the cursor and now — the ring
		// describes a process the client never saw — or the cursor claims a
		// position this process cannot have produced. Replaying it would be
		// worse than replaying nothing: the client must refetch.
		status = protocol.ReplayStale
	case len(o.replay) > 0 && after.Seq+1 < o.replay[0].Seq:
		// The cursor predates the ring; the events in between are gone.
		status = protocol.ReplayGap
	default:
		// Contiguous: deliver only what follows the cursor.
		status = protocol.ReplayResumed
		for _, env := range o.replay {
			if env.Seq > after.Seq {
				ch <- env
			}
		}
	}
	replayed := o.seq
	o.subs[id] = &subscriber{ch: ch}
	return ch, status, replayed, func() {
		o.mu.Lock()
		defer o.mu.Unlock()
		if s, ok := o.subs[id]; ok {
			delete(o.subs, id)
			close(s.ch)
		}
	}
}

// Epoch reports this process's resume epoch.
func (o *Orchestrator) Epoch() string {
	o.mu.RLock()
	defer o.mu.RUnlock()
	return o.epoch
}

// CheckpointView is the UI-facing description of a project's rewind points.
// Available is false for a directory that is not a git repository with at
// least one commit — the normal case for a project that is just a folder, and
// the UI hides the feature rather than showing an error.
type CheckpointView struct {
	ProjectID   string                  `json:"projectID"`
	Available   bool                    `json:"available"`
	Dirty       bool                    `json:"dirty"`
	Checkpoints []checkpoint.Checkpoint `json:"checkpoints"`
}

// Checkpoints lists the stored turns for one session.
func (o *Orchestrator) Checkpoints(projectID, sessionID string) (CheckpointView, error) {
	dir, err := o.projectPath(projectID)
	if err != nil {
		return CheckpointView{}, err
	}
	view := CheckpointView{ProjectID: projectID}
	if !checkpoint.Available(dir) {
		return view, nil
	}
	view.Available = true
	if dirty, err := checkpoint.Dirty(context.Background(), dir); err == nil {
		view.Dirty = dirty
	}
	cps, err := checkpoint.List(context.Background(), dir, sessionID)
	if err != nil {
		return view, err
	}
	view.Checkpoints = cps
	return view, nil
}

// RewindPreview is what a rewind would do, computed without doing it. The UI
// shows this before asking for confirmation, because a rewind discards work.
type RewindPreview struct {
	Ref      string `json:"ref"`
	Turn     int    `json:"turn"`
	Diff     string `json:"diff"`
	WillLose bool   `json:"willLose"` // uncommitted work exists right now
	GuardRef string `json:"guardRef"` // where the current state is preserved
}

// PreviewRewind describes the effect of rewinding a session to one stored
// turn. It is a separate call from Rewind so the confirmation is informed
// rather than a guess.
func (o *Orchestrator) PreviewRewind(projectID, sessionID string, turn int) (RewindPreview, error) {
	dir, err := o.projectPath(projectID)
	if err != nil {
		return RewindPreview{}, err
	}
	if !checkpoint.Available(dir) {
		return RewindPreview{}, &checkpoint.ErrNotARepo{Dir: dir}
	}
	cps, err := checkpoint.List(context.Background(), dir, sessionID)
	if err != nil {
		return RewindPreview{}, err
	}
	var start *checkpoint.Checkpoint
	for i := range cps {
		if cps[i].Turn == turn && cps[i].Kind == checkpoint.KindStart {
			start = &cps[i]
		}
	}
	if start == nil {
		return RewindPreview{}, fmt.Errorf("orchestrator: no checkpoint for session %s turn %d", sessionID, turn)
	}
	// The diff is measured against the latest stored state, which is the
	// current workspace unless later turns exist.
	from := start.Ref
	if len(cps) > 0 {
		if last := cps[len(cps)-1]; last.Kind == checkpoint.KindEnd {
			from = last.Ref
		}
	}
	preview := RewindPreview{Ref: start.Ref, Turn: turn}
	diff, err := checkpoint.Diff(context.Background(), dir, from, start.Ref)
	if err != nil {
		return RewindPreview{}, err
	}
	preview.Diff = diff
	preview.WillLose, _ = checkpoint.Dirty(context.Background(), dir)
	return preview, nil
}

// Rewind restores the workspace to the start of one turn. The state on disk is
// itself checkpointed first, so a rewind can be undone — which is why this is
// not as irreversible as it looks. Callers are expected to have shown the
// preview and obtained confirmation.
func (o *Orchestrator) Rewind(ctx context.Context, projectID, sessionID string, turn int) (CheckpointView, error) {
	preview, err := o.PreviewRewind(projectID, sessionID, turn)
	if err != nil {
		return CheckpointView{}, err
	}
	dir, err := o.projectPath(projectID)
	if err != nil {
		return CheckpointView{}, err
	}
	// Preserve what is on disk right now before overwriting it.
	if _, err := checkpoint.Capture(ctx, dir, sessionID, turn, checkpoint.KindGuard); err != nil {
		return CheckpointView{}, err
	}
	if err := checkpoint.Restore(ctx, dir, preview.Ref); err != nil {
		return CheckpointView{}, err
	}
	return o.Checkpoints(projectID, sessionID)
}

func (o *Orchestrator) projectPath(id string) (string, error) {
	o.mu.RLock()
	defer o.mu.RUnlock()
	p := o.projects[id]
	if p == nil {
		return "", fmt.Errorf("orchestrator: unknown project %s", id)
	}
	return p.cfg.Path, nil
}

// ProjectView is the REST-facing project descriptor.
type ProjectView struct {
	ID       string `json:"id"`
	Path     string `json:"path"`
	Mode     string `json:"mode"`
	Provider string `json:"provider"`
	URL      string `json:"url,omitempty"`
	Status   string `json:"status"` // starting|running|stopped|error
	Detail   string `json:"detail,omitempty"`
}

// AddProject registers and starts a project. Start happens in the background;
// the returned view reports "starting" so the UI can render progress.
// Provider selects the agent backend; empty means "opencode".
func (o *Orchestrator) AddProject(ctx context.Context, path, mode, url, provider string) (ProjectView, error) {
	if mode == "" {
		mode = "managed"
	}
	if mode != "managed" && mode != "attach" {
		return ProjectView{}, fmt.Errorf("orchestrator: unknown mode %q", mode)
	}
	if provider == "" {
		provider = "opencode"
	}
	if provider != "opencode" && provider != "omp" {
		return ProjectView{}, fmt.Errorf("orchestrator: unknown provider %q", provider)
	}
	if provider == "omp" && mode == "attach" {
		return ProjectView{}, fmt.Errorf("orchestrator: omp provider supports managed mode only")
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
		ID:       store.ProjectID(path),
		Path:     path,
		Mode:     mode,
		URL:      url,
		Provider: provider,
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

// SetAccess switches a project's access mode (omp only). omp applies
// tools.approvalMode at process start only, so the adapter records the mode
// and is restarted: Stop → factory → Start. The live session is re-adopted
// by Start's getState; a streaming turn is aborted first so it isn't
// orphaned mid-run. The mode persists in settings.
func (o *Orchestrator) SetAccess(ctx context.Context, id, mode string) error {
	switch mode {
	case protocol.AccessSupervised, protocol.AccessEdits, protocol.AccessFull:
	default:
		return fmt.Errorf("orchestrator: unknown access mode %q", mode)
	}
	o.mu.RLock()
	p := o.projects[id]
	o.mu.RUnlock()
	if p == nil {
		return fmt.Errorf("orchestrator: unknown project %s", id)
	}

	o.mu.Lock()
	adapter := p.adapter
	o.mu.Unlock()
	if adapter == nil {
		return fmt.Errorf("orchestrator: project %s has no adapter", id)
	}
	if !adapter.AccessSupported() {
		return fmt.Errorf("orchestrator: provider %s has no access-mode surface", p.cfg.Provider)
	}
	if p.cfg.AccessMode == mode {
		return nil
	}
	if err := adapter.SetAccess(ctx, mode); err != nil {
		return err
	}

	o.mu.Lock()
	p.cfg.AccessMode = mode
	ready := p.ready
	o.mu.Unlock()
	if err := o.persistLocked(); err != nil {
		return fmt.Errorf("orchestrator: persist access mode: %w", err)
	}
	if ready {
		o.restartAdapter(ctx, id)
	}
	return nil
}

// restartAdapter stops the current adapter (aborting a streaming turn first
// so the turn isn't orphaned) and starts a fresh one via the factory.
func (o *Orchestrator) restartAdapter(ctx context.Context, id string) {
	o.mu.Lock()
	p := o.projects[id]
	old := p.adapter
	p.adapter = nil
	p.ready = false
	p.status = protocol.AdapterStatus{ProjectID: id, State: protocol.AdapterStarting}
	o.mu.Unlock()
	env, err := protocol.NewEnvelope(protocol.EventAdapterStatus, p.status)
	if err != nil {
		env = protocol.Envelope{Type: protocol.EventAdapterStatus}
	}
	o.broadcast(env)

	if old != nil {
		_ = old.Stop(context.Background())
	}
	o.startAdapter(ctx, id)
}

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
			ID:       p.cfg.ID,
			Path:     p.cfg.Path,
			Mode:     p.cfg.Mode,
			Provider: p.cfg.Provider,
			URL:      p.cfg.URL,
			Status:   p.status.State,
			Detail:   p.status.Detail,
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
		ID:       p.cfg.ID,
		Path:     p.cfg.Path,
		Mode:     p.cfg.Mode,
		Provider: p.cfg.Provider,
		URL:      p.cfg.URL,
		Status:   p.status.State,
		Detail:   p.status.Detail,
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
