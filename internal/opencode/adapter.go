package opencode

import (
	"context"
	"encoding/json"
	"fmt"
	"path/filepath"
	"sort"
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
	Binary    string // managed: default "opencode" (CIRCULOGO_OPENCODE_BIN)

	// Optional Basic auth. Managed v2 servers print their boot password and
	// require it on every route; attach needs the user-provided password.
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

	// rootDir is the server's resolved scope root (GET /api/location →
	// directory). The OpenCode session store is global on disk: the session
	// list and the event stream carry sessions from EVERY directory this
	// user ever ran opencode in, so scoping happens here — the adapter only
	// lets through sessions whose directory equals this root.
	rootDir string

	mu         sync.Mutex
	managed    *Managed
	procCancel context.CancelFunc // cancels the managed process context
	cancel     context.CancelFunc // cancels the SSE loop
	stopped    bool
}

// compile-time proof the adapter satisfies the contract (AGENTS.md rule).
var _ agent.Adapter = (*Adapter)(nil)

// ErrNotReady is returned by data methods called before Start finished
// wiring the HTTP client.
var ErrNotReady = fmt.Errorf("opencode: adapter not started")

// clientOrErr returns the wired client, or ErrNotReady. Public methods must
// not touch a.client directly: the orchestrator registers the adapter before
// Start completes, so requests can legitimately arrive early.
func (a *Adapter) clientOrErr() (*Client, error) {
	a.mu.Lock()
	defer a.mu.Unlock()
	if a.client == nil {
		return nil, ErrNotReady
	}
	return a.client, nil
}

// NewAdapter builds an adapter; call Start before using it.
func NewAdapter(cfg AdapterConfig) *Adapter {
	return &Adapter{
		cfg:    cfg,
		events: make(chan protocol.Envelope, 1024),
	}
}

func (a *Adapter) Events() <-chan protocol.Envelope { return a.events }

// Start brings the backend up and begins consuming /api/event.
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
		if _, err := a.client.ServerInfoV2(hctx); err != nil {
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
		// v2 prints its boot password and requires Basic auth with it; the
		// authed client only exists once both are true.
		c, err := m.WaitReady(ctx, func(pass string) *Client {
			return NewClient(url, WithBasicAuth("opencode", pass))
		}, 10*time.Second)
		if err != nil {
			m.Stop(2 * time.Second)
			procCancel()
			a.mu.Lock()
			a.managed = nil
			a.procCancel = nil
			a.mu.Unlock()
			return err
		}
		a.client = c

	default:
		return fmt.Errorf("opencode: unknown adapter mode %q", a.cfg.Mode)
	}

	// Resolve the server's scope root. GET /api/location is the source of
	// truth (its directory is symlink-resolved, e.g. /tmp/x → /private/tmp/x
	// on macOS); fall back to the configured project dir resolved the same.
	a.rootDir = a.cfg.Dir
	if pctx, pcancel := context.WithTimeout(context.Background(), 5*time.Second); pcancel != nil {
		defer pcancel()
		if p, err := a.client.LocationV2(pctx); err == nil && p.Directory != "" {
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

// reconnectHysteresis: transient stream blips (brief proxy hiccups, server
// restarts) must NOT flip the project banner to error — the loop reconnects
// and server.connected restores running within a second. Only surface an
// error after this many consecutive failed attempts.
const reconnectHysteresis = 3

// sseLoop consumes /api/event with reconnect until ctx is cancelled.
// Reconnect is safe because stream *ended frames carry the full cumulative
// text and tool updates are idempotent by call id (self-healing).
func (a *Adapter) sseLoop(ctx context.Context) {
	defer close(a.events)
	consecutiveFailures := 0
	for {
		if ctx.Err() != nil {
			return
		}
		body, err := a.client.EventsV2(ctx)
		if err != nil {
			if ctx.Err() != nil {
				return
			}
			consecutiveFailures++
			if consecutiveFailures >= reconnectHysteresis {
				a.emitStatus(protocol.AdapterError, err.Error())
			}
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
				consecutiveFailures = 0
				env, err := parseV2Event(f)
				if err != nil {
					// One malformed frame must not kill the stream.
					continue
				}
				if a.foreignSessionEvent(env) {
					continue
				}
				out, err := TranslateV2(a.cfg.ProjectID, env)
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
				consecutiveFailures++
				if consecutiveFailures >= reconnectHysteresis {
					a.emitStatus(protocol.AdapterError, "event stream dropped: "+err.Error())
				}
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

// foreignSessionEvent reports whether a raw v2 event carries a session
// scoped to another directory. Only session.created carries the location —
// partial patches (renamed, streams) are translated for every session and
// filtered by the reducer, which never creates a session from a patch.
func (a *Adapter) foreignSessionEvent(env V2Event) bool {
	if a.rootDir == "" || env.Type != "session.created" {
		return false
	}
	var p struct {
		Location struct {
			Directory string `json:"directory"`
		} `json:"location"`
	}
	if err := json.Unmarshal(env.Data, &p); err != nil {
		return false // undecodable: let Translate decide
	}
	return !a.inScope(p.Location.Directory)
}

func (a *Adapter) emitStatus(state, detail string) {
	a.emit(protocol.EventAdapterStatus, protocol.AdapterStatus{
		ProjectID: a.cfg.ProjectID,
		State:     state,
		Detail:    detail,
	})
}

// emit sends to the buffered channel. If the consumer is wedged and the buffer
// is full, the event is dropped: ended-frame semantics make the next update
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
	c, err := a.clientOrErr()
	if err != nil {
		return nil, err
	}
	ss, err := c.SessionsV2(ctx)
	if err != nil {
		return nil, err
	}
	out := make([]protocol.Session, 0, len(ss))
	for _, s := range ss {
		if !a.inScope(s.Location.Directory) {
			continue // global store: drop sessions from other directories
		}
		out = append(out, sessionV2ToNeutral(s))
	}
	return out, nil
}

// formatInstruction is attached to every session the app creates: v2 ignores
// the config "instructions" field (docs: "use AGENTS.md"), and the app must
// not write into the user's project — the session instructions-entries API is
// the app-owned path (verified live: the entry rides the turn's instruction
// assembly and reaches the model).
const formatInstructionKey = "circulogo-format"
const formatInstruction = "Format every response in GitHub-flavored Markdown: " +
	"short paragraphs, bullet lists, numbered steps for procedures, tables for " +
	"comparisons, and fenced code blocks with a language tag. Never reply with " +
	"unstructured plain text. When you need to show a flowchart or workflow, " +
	"emit one fenced block labeled circulogo-flow whose body is valid JSON: " +
	`{"nodes":[{"id":"a","row":0,"x":0.5,"w":300,"kind":"Trigger","hue":"purple",` +
	`"title":"New order","caption":"Trigger when an order is created"}],` +
	`"edges":[{"from":"a","to":"b"}]}. ` +
	"row = depth from top starting at 0; x = horizontal center from 0 to 1; " +
	"hue: purple|amber|blue|green|red; a decision node uses kind \"If / Else\" " +
	`and adds "condition":[["order.flavor","is","Rocky Road"]] (read-only rows). " +
	"Keep the JSON strictly valid, no comments."

func (a *Adapter) CreateSession(ctx context.Context, title string) (protocol.Session, error) {
	c, err := a.clientOrErr()
	if err != nil {
		return protocol.Session{}, err
	}
	s, err := c.CreateSessionV2(ctx, title)
	if err != nil {
		return protocol.Session{}, err
	}
	// Best-effort guidance: the entries endpoint is experimental (spec
	// /api/experimental/...), and an unformatted reply is degraded UX, not a
	// failed session — dropping the error is deliberate here.
	_ = c.PutInstructionV2(ctx, s.ID, formatInstructionKey, formatInstruction)
	return sessionV2ToNeutral(s), nil
}

func (a *Adapter) RenameSession(ctx context.Context, sessionID, title string) error {
	c, err := a.clientOrErr()
	if err != nil {
		return err
	}
	return c.RenameSessionV2(ctx, sessionID, title)
}

func (a *Adapter) DeleteSession(ctx context.Context, sessionID string) error {
	c, err := a.clientOrErr()
	if err != nil {
		return err
	}
	return c.DeleteSessionV2(ctx, sessionID)
}

func (a *Adapter) Messages(ctx context.Context, sessionID string, limit int) ([]agent.HydratedMessage, error) {
	c, err := a.clientOrErr()
	if err != nil {
		return nil, err
	}
	msgs, err := c.MessagesV2(ctx, sessionID, limit)
	if err != nil {
		return nil, err
	}
	return messagesV2ToNeutral(sessionID, msgs), nil
}

func (a *Adapter) Prompt(ctx context.Context, sessionID string, req protocol.PromptRequest) error {
	c, err := a.clientOrErr()
	if err != nil {
		return err
	}
	// v2 scopes model/agent to the session: pin them first when the request
	// selects any, then send the text-only prompt.
	if req.Provider != "" || req.Model != "" || req.Variant != "" {
		if err := c.SetModelV2(ctx, sessionID, req.Provider, req.Model, req.Variant); err != nil {
			return err
		}
	}
	if req.Agent != "" {
		if err := c.SetAgentV2(ctx, sessionID, req.Agent); err != nil {
			return err
		}
	}
	return c.PromptV2(ctx, sessionID, req.Text)
}

func (a *Adapter) Abort(ctx context.Context, sessionID string) error {
	c, err := a.clientOrErr()
	if err != nil {
		return err
	}
	return c.InterruptV2(ctx, sessionID)
}

func (a *Adapter) ReplyPermission(ctx context.Context, sessionID, permissionID, response string) error {
	c, err := a.clientOrErr()
	if err != nil {
		return err
	}
	return c.ReplyPermissionV2(ctx, sessionID, permissionID, response)
}

// ReplyForm answers a pending form (question tool) via the form reply API.
func (a *Adapter) ReplyForm(ctx context.Context, sessionID, formID string, answer map[string]any) error {
	c, err := a.clientOrErr()
	if err != nil {
		return err
	}
	return c.ReplyFormV2(ctx, sessionID, formID, answer)
}

func (a *Adapter) Meta(ctx context.Context) (protocol.Meta, error) {
	c, err := a.clientOrErr()
	if err != nil {
		return protocol.Meta{}, err
	}
	agents, err := c.AgentsV2(ctx)
	if err != nil {
		return protocol.Meta{}, err
	}
	models, err := c.ModelsV2(ctx)
	if err != nil {
		return protocol.Meta{}, err
	}
	meta := protocol.Meta{Agents: []protocol.AgentInfo{}, Models: []protocol.ModelInfo{}}
	for _, ag := range agents {
		if ag.Hidden {
			continue
		}
		meta.Agents = append(meta.Agents, protocol.AgentInfo{
			Name:        ag.ID,
			Description: ag.Description,
			Mode:        ag.Mode,
		})
	}
	for _, m := range models {
		if !m.Enabled {
			continue
		}
		variants := make([]string, 0, len(m.Variants))
		for _, v := range m.Variants {
			variants = append(variants, v.ID)
		}
		sort.Strings(variants)
		meta.Models = append(meta.Models, protocol.ModelInfo{
			ID:        m.ID,
			Name:      m.Name,
			Provider:  m.ProviderID,
			Reasoning: len(variants) > 0,
			Variants:  variants,
		})
	}
	// Surface the server's configured default model so the composer doesn't
	// guess (the first list entry may be a plan-restricted variant).
	if dm, err := c.DefaultModelV2(ctx); err == nil && dm != nil {
		meta.DefaultProvider = dm.ProviderID
		meta.DefaultModel = dm.ID
	}
	return meta, nil
}

// sessionV2ToNeutral maps a v2 session onto the neutral contract.
func sessionV2ToNeutral(s V2Session) protocol.Session {
	return protocol.Session{
		ID:          s.ID,
		Title:       s.Title,
		Directory:   s.Location.Directory,
		TimeCreated: s.Time.Created,
		TimeUpdated: s.Time.Updated,
	}
}

// messagesV2ToNeutral converts the v2 message list (newest-first, content
// inline, idle/system markers interleaved) into chronological neutral
// hydrated messages. Part ids match the live stream synthesis
// (<messageID>:text|:reasoning, tool call ids, <messageID>:finish).
func messagesV2ToNeutral(sessionID string, msgs []V2Message) []agent.HydratedMessage {
	out := make([]agent.HydratedMessage, 0, len(msgs))
	for i := len(msgs) - 1; i >= 0; i-- { // oldest-first
		m := msgs[i]
		switch m.Type {
		case "user":
			out = append(out, agent.HydratedMessage{
				Info: protocol.MessageInfo{
					ID:        m.ID,
					SessionID: sessionID,
					Role:      protocol.RoleUser,
					Created:   m.Time.Created,
				},
				Parts: []protocol.Part{{
					ID:   m.ID + ":text",
					Type: protocol.PartText,
					Text: m.Text,
				}},
			})
		case "assistant":
			info := protocol.MessageInfo{
				ID:        m.ID,
				SessionID: sessionID,
				Role:      protocol.RoleAssistant,
				Created:   m.Time.Created,
				Completed: m.Time.Completed,
				Agent:     m.Agent,
				Provider:  m.Model.ProviderID,
				Model:     m.Model.ID,
				Cost:      m.Cost,
				Finish:    m.Finish,
			}
			if m.Tokens != nil {
				info.Tokens = &protocol.TokenUsage{
					Input:      m.Tokens.Input,
					Output:     m.Tokens.Output,
					Reasoning:  m.Tokens.Reasoning,
					CacheRead:  m.Tokens.Cache.Read,
					CacheWrite: m.Tokens.Cache.Write,
				}
			}
			parts := make([]protocol.Part, 0, len(m.Content)+1)
			for _, c := range m.Content {
				switch c.Type {
				case "text":
					parts = append(parts, protocol.Part{
						ID:   m.ID + ":text",
						Type: protocol.PartText,
						Text: c.Text,
					})
				case "reasoning":
					parts = append(parts, protocol.Part{
						ID:   m.ID + ":reasoning",
						Type: protocol.PartReasoning,
						Text: c.Text,
					})
				case "tool":
					parts = append(parts, protocol.Part{
						ID:     c.ID,
						Type:   protocol.PartTool,
						CallID: c.ID,
						Tool:   c.Name,
						State: &protocol.ToolState{
							Status: toolStatusV2(c.State.Status),
							Input:  c.State.Input,
							Output: c.stateOutput(),
							Error:  c.stateError(),
						},
					})
				}
			}
			if m.Finish != "" || m.Cost > 0 || m.Tokens != nil {
				finish := protocol.Part{
					ID:     m.ID + ":finish",
					Type:   protocol.PartStepFinish,
					Reason: m.Finish,
					Cost:   m.Cost,
				}
				if m.Tokens != nil {
					finish.Tokens = &protocol.TokenUsage{
						Input:      m.Tokens.Input,
						Output:     m.Tokens.Output,
						Reasoning:  m.Tokens.Reasoning,
						CacheRead:  m.Tokens.Cache.Read,
						CacheWrite: m.Tokens.Cache.Write,
					}
				}
				parts = append(parts, finish)
			}
			out = append(out, agent.HydratedMessage{Info: info, Parts: parts})
		default:
			// idle/system markers: turn boundaries and server context notes
			// have no renderer — dropped in v0.
		}
	}
	return out
}

// toolStatusV2 maps a v2 tool state status onto the neutral vocabulary.
func toolStatusV2(status string) string {
	switch status {
	case "completed":
		return protocol.ToolCompleted
	case "error":
		return protocol.ToolError
	case "running", "streaming":
		return protocol.ToolRunning
	default:
		return protocol.ToolPending
	}
}
