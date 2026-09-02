package opencode

import (
	"context"
	"encoding/json"
	"fmt"
	"path/filepath"
	"sync"
	"time"

	"circulogo/internal/agent"
	"circulogo/internal/agent/protocol"
)

// resolveDir returns the symlink-resolved absolute path of dir (empty in →
// empty out). macOS temp dirs resolve /tmp → /private/tmp; the server
// reports directories in resolved form, so comparisons need this.
func resolveDir(dir string) (string, error) {
	if dir == "" {
		return "", nil
	}
	abs, err := filepath.Abs(dir)
	if err != nil {
		return "", err
	}
	if resolved, err := filepath.EvalSymlinks(abs); err == nil {
		return resolved, nil
	}
	return abs, nil
}

// inScope reports whether a session directory belongs to this adapter's
// project. The server reports directories in resolved form (it resolved the
// symlink when creating the session), so a lexical comparison is correct and
// never touches the filesystem — session dirs may no longer exist on disk.
// Empty rootDir (root unresolved) disables filtering.
func (a *Adapter) inScope(sessionDir string) bool {
	if a.rootDir == "" {
		return true
	}
	if sessionDir == "" {
		return false // server always sets it; treat missing as foreign
	}
	return filepath.Clean(sessionDir) == filepath.Clean(a.rootDir)
}

// AdapterConfig configures one project's OpenCode adapter.
type AdapterConfig struct {
	ProjectID string
	Mode      string // "managed" (spawn) | "attach" (existing server)
	Dir       string // managed: project directory the server is rooted at
	URL       string // attach: base URL of the running server
	Binary    string // managed: default "opencode"

	// Optional Basic auth (OPENCODE_SERVER_PASSWORD on the server).
	Username string
	Password string
}

// Adapter modes.
const (
	ModeManaged = "managed"
	ModeAttach  = "attach"
)

// Adapter implements agent.Adapter for OpenCode over HTTP+SSE (no ACP).
type Adapter struct {
	cfg    AdapterConfig
	client *Client
	events chan protocol.Envelope

	// rootDir is the server's resolved scope root (GET /path → directory).
	// The OpenCode session store is global on disk: GET /session and the
	// event stream carry sessions from EVERY directory this user ever ran
	// opencode in, so scoping happens here — the adapter only lets through
	// sessions whose Directory equals this root.
	rootDir string

	mu      sync.Mutex
	managed  *Managed
	procCancel context.CancelFunc // cancels the managed process context
	cancel   context.CancelFunc // cancels the SSE loop
	stopped bool
}

// compile-time proof the adapter satisfies the contract (AGENTS.md rule).
var _ agent.Adapter = (*Adapter)(nil)

// NewAdapter builds an adapter; call Start before using it.
func NewAdapter(cfg AdapterConfig) *Adapter {
	return &Adapter{
		cfg:    cfg,
		events: make(chan protocol.Envelope, 1024),
	}
}

func (a *Adapter) Events() <-chan protocol.Envelope { return a.events }

// Start brings the backend up and begins consuming /event.
func (a *Adapter) Start(ctx context.Context) error {
	opts := []ClientOption{}
	if a.cfg.Password != "" {
		opts = append(opts, WithBasicAuth(a.cfg.Username, a.cfg.Password))
	}
	url := a.cfg.URL

	switch a.cfg.Mode {
	case ModeAttach:
		if url == "" {
			return fmt.Errorf("opencode: attach mode requires URL")
		}
		a.client = NewClient(url, opts...)
		hctx, cancel := context.WithTimeout(ctx, 5*time.Second)
		defer cancel()
		if _, err := a.client.Health(hctx); err != nil {
			return fmt.Errorf("opencode: attach health check failed: %w", err)
		}

	case ModeManaged:
		port, err := FreePort()
		if err != nil {
			return fmt.Errorf("opencode: no free port: %w", err)
		}
		// The process is tied to the adapter's lifetime (Stop), not to the
		// caller's Start ctx — a short-lived request context must not be able
		// to kill the project's server.
		procCtx, procCancel := context.WithCancel(context.Background())
		m, err := StartManaged(procCtx, a.cfg.Binary, a.cfg.Dir, port)
		if err != nil {
			procCancel()
			return err
		}
		a.mu.Lock()
		a.managed = m
		a.procCancel = procCancel
		a.mu.Unlock()
		url = fmt.Sprintf("http://127.0.0.1:%d", port)
		a.client = NewClient(url, opts...)
		if err := m.WaitHealthy(ctx, a.client, 10*time.Second); err != nil {
			m.Stop(2 * time.Second)
			procCancel()
			a.mu.Lock()
			a.managed = nil
			a.procCancel = nil
			a.mu.Unlock()
			return err
		}

	default:
		return fmt.Errorf("opencode: unknown adapter mode %q", a.cfg.Mode)
	}

	// Resolve the server's scope root. GET /path is the source of truth (its
	// directory is symlink-resolved, e.g. /tmp/x → /private/tmp/x on macOS);
	// fall back to the configured project dir resolved the same way.
	a.rootDir = a.cfg.Dir
	if pctx, pcancel := context.WithTimeout(context.Background(), 5*time.Second); pcancel != nil {
		defer pcancel()
		if p, err := a.client.Path(pctx); err == nil && p.Directory != "" {
			a.rootDir = p.Directory
		} else if resolved, rerr := resolveDir(a.cfg.Dir); rerr == nil {
			a.rootDir = resolved
		}
	}

	loopCtx, cancel := context.WithCancel(context.Background())
	a.mu.Lock()
	if a.stopped { // Stop raced with Start
		cancel()
		a.mu.Unlock()
		return fmt.Errorf("opencode: adapter stopped during Start")
	}
	a.cancel = cancel
	a.mu.Unlock()

	go a.sseLoop(loopCtx)

	a.emit(protocol.EventAdapterStatus, protocol.AdapterStatus{
		ProjectID: a.cfg.ProjectID,
		State:     protocol.AdapterRunning,
		Detail:    url,
	})
	return nil
}

// Stop terminates the backend and closes the event channel. Idempotent.
func (a *Adapter) Stop(_ context.Context) error {
	a.mu.Lock()
	if a.stopped {
		a.mu.Unlock()
		return nil
	}
	a.stopped = true
	cancel := a.cancel
	managed := a.managed
	procCancel := a.procCancel
	a.managed = nil
	a.procCancel = nil
	a.mu.Unlock()

	if cancel != nil {
		cancel() // ends sseLoop; it closes a.events
	} else {
		close(a.events)
	}
	if managed != nil {
		managed.Stop(2 * time.Second)
	}
	if procCancel != nil {
		procCancel()
	}
	return nil
}

// sseLoop consumes /event with reconnect until ctx is cancelled. Reconnect is
// safe because every message.part.updated carries the full part (self-healing,
// docs/trd.md §3.3).
func (a *Adapter) sseLoop(ctx context.Context) {
	defer close(a.events)
	for {
		if ctx.Err() != nil {
			return
		}
		body, err := a.client.Events(ctx)
		if err != nil {
			if ctx.Err() != nil {
				return
			}
			// The server may be restarting; surface and retry.
			a.emitStatus(protocol.AdapterError, err.Error())
			select {
			case <-ctx.Done():
				return
			case <-time.After(time.Second):
			}
			continue
		}

		frames := make(chan Frame, 256)
		readErr := make(chan error, 1)
		go func() { readErr <- ReadFrames(body, frames) }()

		done := false
		for !done {
			select {
			case f := <-frames:
				env, err := DecodeEvent(f)
				if err != nil {
					// One malformed frame must not kill the stream.
					continue
				}
				if a.foreignSessionEvent(env) {
					continue
				}
				out, err := Translate(a.cfg.ProjectID, env)
				if err != nil {
					continue
				}
				for _, e := range out {
					select {
					case a.events <- e:
					case <-ctx.Done():
						body.Close()
						return
					}
				}
			case err := <-readErr:
				body.Close()
				if ctx.Err() != nil {
					return
				}
				a.emitStatus(protocol.AdapterError, "event stream dropped: "+err.Error())
				done = true
			case <-ctx.Done():
				body.Close()
				return
			}
		}

		// Backoff before reconnecting.
		select {
		case <-ctx.Done():
			return
		case <-time.After(time.Second):
		}
	}
}

// foreignSessionEvent reports whether a raw OpenCode event carries a session
// scoped to another directory (session.created/updated/deleted include the
// full session). The event stream is user-global, so without this filter the
// sidebar would fill with every session the user ever had anywhere.
func (a *Adapter) foreignSessionEvent(env Envelope) bool {
	if a.rootDir == "" {
		return false
	}
	switch env.Type {
	case "session.created", "session.updated", "session.deleted":
	default:
		return false
	}
	var p EventPropertiesSession
	if err := json.Unmarshal(env.Properties, &p); err != nil {
		return false // undecodable: let Translate decide
	}
	return !a.inScope(p.Info.Directory)
}

func (a *Adapter) emitStatus(state, detail string) {
	a.emit(protocol.EventAdapterStatus, protocol.AdapterStatus{
		ProjectID: a.cfg.ProjectID,
		State:     state,
		Detail:    detail,
	})
}

// emit sends to the buffered channel. If the consumer is wedged and the buffer
// is full, the event is dropped: full-part semantics make the next update
// authoritative, and the relay resyncs sessions on reconnect (flow.md §7).
func (a *Adapter) emit(typ string, payload any) {
	env, err := protocol.NewEnvelope(typ, payload)
	if err != nil {
		return
	}
	select {
	case a.events <- env:
	default:
	}
}

// --- agent.Adapter data methods ---

func (a *Adapter) Sessions(ctx context.Context) ([]protocol.Session, error) {
	ss, err := a.client.Sessions(ctx)
	if err != nil {
		return nil, err
	}
	out := make([]protocol.Session, 0, len(ss))
	for _, s := range ss {
		if !a.inScope(s.Directory) {
			continue // global store: drop sessions from other directories
		}
		out = append(out, sessionToNeutral(s))
	}
	return out, nil
}

func (a *Adapter) CreateSession(ctx context.Context, title string) (protocol.Session, error) {
	s, err := a.client.CreateSession(ctx, title)
	if err != nil {
		return protocol.Session{}, err
	}
	return sessionToNeutral(s), nil
}

func (a *Adapter) RenameSession(ctx context.Context, sessionID, title string) error {
	return a.client.RenameSession(ctx, sessionID, title)
}

func (a *Adapter) DeleteSession(ctx context.Context, sessionID string) error {
	return a.client.DeleteSession(ctx, sessionID)
}

func (a *Adapter) Messages(ctx context.Context, sessionID string, limit int) ([]agent.HydratedMessage, error) {
	pages, err := a.client.Messages(ctx, sessionID, limit)
	if err != nil {
		return nil, err
	}
	out := make([]agent.HydratedMessage, 0, len(pages))
	for _, pg := range pages {
		hm := agent.HydratedMessage{Info: messageToNeutral(pg.Info)}
		for _, p := range pg.Parts {
			np, ok := partToNeutral(p)
			if !ok {
				continue
			}
			hm.Parts = append(hm.Parts, np)
		}
		out = append(out, hm)
	}
	return out, nil
}

func (a *Adapter) Prompt(ctx context.Context, sessionID string, req protocol.PromptRequest) error {
	return a.client.Prompt(ctx, sessionID, PromptInput{
		Text:     req.Text,
		Agent:    req.Agent,
		Provider: req.Provider,
		Model:    req.Model,
	})
}

func (a *Adapter) Abort(ctx context.Context, sessionID string) error {
	return a.client.Abort(ctx, sessionID)
}

func (a *Adapter) ReplyPermission(ctx context.Context, sessionID, permissionID, response string) error {
	return a.client.ReplyPermission(ctx, sessionID, permissionID, response)
}

func (a *Adapter) Meta(ctx context.Context) (protocol.Meta, error) {
	agents, err := a.client.Agents(ctx)
	if err != nil {
		return protocol.Meta{}, err
	}
	providers, err := a.client.Providers(ctx)
	if err != nil {
		return protocol.Meta{}, err
	}
	meta := protocol.Meta{Agents: []protocol.AgentInfo{}, Models: []protocol.ModelInfo{}}
	for _, ag := range agents {
		if ag.Hidden {
			continue
		}
		meta.Agents = append(meta.Agents, protocol.AgentInfo{
			Name:        ag.Name,
			Description: ag.Description,
			Mode:        ag.Mode,
		})
	}
	for _, p := range providers.Providers {
		for _, m := range p.Models {
			meta.Models = append(meta.Models, protocol.ModelInfo{
				ID:       m.ID,
				Name:     m.Name,
				Provider: p.ID,
			})
		}
	}
	// Surface the server's configured default model so the composer doesn't
	// guess (the first list entry may be a plan-restricted variant).
	for providerID, modelID := range providers.Default {
		meta.DefaultProvider = providerID
		meta.DefaultModel = modelID
		break // one default pair is all the UI needs
	}
	return meta, nil
}
