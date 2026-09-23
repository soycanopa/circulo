package omp

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"os/exec"
	"strings"
	"sync"
	"time"

	"circulogo/internal/agent"
	"circulogo/internal/agent/protocol"
)

// Compile proof: *Adapter satisfies the neutral adapter contract.
var _ agent.Adapter = (*Adapter)(nil)

// AdapterConfig configures one project's omp adapter.
type AdapterConfig struct {
	ProjectID string
	// Dir is the project root: the omp child runs with cwd = Dir, so its
	// session bucket and all tool work are rooted here.
	Dir string
	// Binary overrides the omp executable (CIRCULOGO_OMP_BIN in main.go).
	Binary string
	// Command replaces the child invocation entirely (tests).
	Command []string
	// AccessMode is the neutral access mode (protocol.Access*); mapped onto
	// omp's --approval-mode at spawn. Empty = omp's own default (yolo).
	AccessMode string
}

// Adapter drives one `omp --mode rpc` child and translates its JSONL frames
// into neutral protocol envelopes. omp has no network server: lifecycle is
// managed-only (spawn/kill), one live session per process.
type Adapter struct {
	cfg    AdapterConfig
	tr     *translator
	events chan protocol.Envelope

	mu        sync.Mutex
	proc      *process
	client    *rpcClient
	sessionID string
	streaming bool
	formKinds map[string]string // form id -> omp ui method
	stopped   bool
}

// NewAdapter builds an adapter for one project.
func NewAdapter(cfg AdapterConfig) *Adapter {
	return &Adapter{
		cfg:       cfg,
		tr:        newTranslator(cfg.ProjectID),
		events:    make(chan protocol.Envelope, 1024),
		formKinds: map[string]string{},
	}
}

// Start spawns the omp child, completes the RPC handshake, and emits
// adapter.status running. The child outlives the caller's context: only Stop
// ends it (orchestrator startAdapter contract).
//
// Locking: a.mu is taken only for field assignments. Start must never hold it
// across getState/call — call locks a.mu itself (self-deadlock otherwise).
func (a *Adapter) Start(_ context.Context) error {
	a.mu.Lock()
	if a.proc != nil {
		a.mu.Unlock()
		return fmt.Errorf("omp: adapter already started")
	}

	proc, err := a.spawn()
	if err != nil {
		a.mu.Unlock()
		return err
	}
	client := newRPCClient(proc.stdin)
	client.OnEvent = a.onFrame
	a.proc = proc
	a.client = client
	a.mu.Unlock()

	// Watch for child death while running: surface it as adapter.status error
	// unless Stop closed the stream first.
	go a.watchChild()
	go client.ReadLoop(proc.stdout)

	readyTimeout := 15 * time.Second
	select {
	case <-client.Ready:
	case err := <-waitErr(client):
		return fmt.Errorf("omp: startup failed: %v; output: %s", err, proc.errTail())
	case <-time.After(readyTimeout):
		return fmt.Errorf("omp: startup timed out after %s; output: %s", readyTimeout, proc.errTail())
	}
	client.negotiate(context.Background())

	// Adopt the current session so later prompts hit the right target.
	state, err := a.getState(context.Background())
	if err != nil {
		return err
	}
	a.mu.Lock()
	a.sessionID = state.SessionID
	a.streaming = state.IsStreaming
	a.mu.Unlock()

	a.emitStatus(protocol.AdapterRunning, "")
	return nil
}

// spawn builds and starts the omp child process.
func (a *Adapter) spawn() (*process, error) {
	if len(a.cfg.Command) > 0 {
		return startCommand(a.cfg.Command[0], a.cfg.Command[1:], a.cfg.Dir)
	}
	bin := a.cfg.Binary
	if bin == "" {
		bin = "omp"
	}
	return startProcess(bin, a.cfg.Dir, approvalModeFor(a.cfg.AccessMode))
}

// approvalModeFor maps a neutral access mode onto omp's --approval-mode
// values. Unknown/empty returns "" (omp's own default: yolo).
func approvalModeFor(accessMode string) string {
	switch accessMode {
	case protocol.AccessSupervised:
		return "always-ask"
	case protocol.AccessEdits:
		return "write"
	case protocol.AccessFull:
		return "yolo"
	default:
		return ""
	}
}

// accessMode normalizes the configured mode for reporting: empty (omp's
// default) reports as full, unknown values report as full too (fail open on
// display only; spawn still passes them through untouched).
func accessMode(configured string) string {
	switch configured {
	case protocol.AccessSupervised, protocol.AccessEdits:
		return configured
	default:
		return protocol.AccessFull
	}
}

// AccessSupported: omp exposes an access-mode surface via --approval-mode.
func (a *Adapter) AccessSupported() bool { return true }

// SetAccess records the mode; the child picks it up on the next spawn. The
// mode only takes effect at process start (omp has no RPC setter), so the
// orchestrator restarts the adapter after calling this.
func (a *Adapter) SetAccess(_ context.Context, mode string) error {
	switch mode {
	case protocol.AccessSupervised, protocol.AccessEdits, protocol.AccessFull:
	default:
		return fmt.Errorf("omp: unknown access mode %q", mode)
	}
	a.mu.Lock()
	defer a.mu.Unlock()
	a.cfg.AccessMode = mode
	return nil
}

// accessModes is the composer picker data for omp. Waku-style tiers without
// Waku's AI-reviewer mode: omp has no equivalent.
func accessModes() []protocol.AccessModeInfo {
	return []protocol.AccessModeInfo{
		{
			ID:          protocol.AccessSupervised,
			Title:       "Supervised",
			Description: "Ask before commands and file changes",
		},
		{
			ID:          protocol.AccessEdits,
			Title:       "Auto-accept edits",
			Description: "Auto-approve edits, ask before other actions",
		},
		{
			ID:          protocol.AccessFull,
			Title:       "Full access",
			Description: "Allow commands and edits without prompts",
		},
	}
}

// waitErr adapts client.Done into an error channel.
func waitErr(c *rpcClient) <-chan error {
	ch := make(chan error, 1)
	go func() {
		<-c.Done
		ch <- c.doneErr
	}()
	return ch
}

// watchChild reports unexpected child exit as adapter.status error. Stop
// marks stopped first, so orderly shutdown stays silent.
func (a *Adapter) watchChild() {
	c := a.client
	<-c.Done
	a.mu.Lock()
	stopped := a.stopped
	a.mu.Unlock()
	if stopped {
		return
	}
	detail := c.doneErr.Error()
	if a.proc != nil {
		detail += "; output: " + a.proc.errTail()
	}
	a.emitStatus(protocol.AdapterError, detail)
}

// onFrame routes one non-response stdout frame: extension UI bookkeeping,
// then translation into neutral envelopes (drop-on-full emit).
func (a *Adapter) onFrame(raw json.RawMessage, typ frameType) {
	if typ == frameExtensionUIRequest {
		var req extensionUIRequest
		if json.Unmarshal(raw, &req) == nil {
			a.mu.Lock()
			a.formKinds[req.ID] = req.Method
			a.mu.Unlock()
		}
	}
	a.mu.Lock()
	sessionID := a.sessionID
	a.mu.Unlock()
	if sessionID == "" {
		return
	}

	envs, err := a.tr.translate(raw, typ, sessionID)
	if err != nil {
		// A malformed vendor frame is logged via the status channel detail —
		// never fatal (forward compatibility).
		return
	}
	for _, env := range envs {
		a.emit(env)
	}
	// turn_end closes a run: the context fill only changes across turns, so
	// one get_state per turn (not per frame) keeps the gauge current.
	if typ == frameTurnEnd {
		go func() {
			ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
			defer cancel()
			_, _ = a.getState(ctx)
		}()
	}
}

func (a *Adapter) emit(env protocol.Envelope) {
	select {
	case a.events <- env:
	default:
		// Slow consumer: drop rather than block the read loop. Full part
		// updates make missed events self-healing on the next resync.
	}
}

func (a *Adapter) emitStatus(state, detail string) {
	env, err := protocol.NewEnvelope(protocol.EventAdapterStatus, protocol.AdapterStatus{
		ProjectID: a.cfg.ProjectID,
		State:     state,
		Detail:    detail,
	})
	if err != nil {
		return
	}
	a.emit(env)
}

// Stop terminates the child and closes the event stream. Idempotent.
func (a *Adapter) Stop(_ context.Context) error {
	a.mu.Lock()
	if a.stopped {
		a.mu.Unlock()
		return nil
	}
	a.stopped = true
	proc := a.proc
	a.mu.Unlock()

	if proc != nil {
		// Close stdin: omp's orderly shutdown drains pending output and
		// exits 0 (rpc.md). Kill only if it hangs.
		proc.stop(2 * time.Second)
	}
	close(a.events)
	return nil
}

// Events yields neutral envelopes until Stop.
func (a *Adapter) Events() <-chan protocol.Envelope { return a.events }

// --- session data -----------------------------------------------------------

// Sessions lists the project's sessions from the omp session bucket on disk,
// newest first.
func (a *Adapter) Sessions(_ context.Context) ([]protocol.Session, error) {
	found, err := DiscoverSessions(a.cfg.Dir)
	if err != nil {
		return nil, err
	}
	out := make([]protocol.Session, 0, len(found))
	for _, s := range found {
		out = append(out, s.Session)
	}
	return out, nil
}

// CreateSession starts a fresh omp session, optionally titled.
func (a *Adapter) CreateSession(ctx context.Context, title string) (protocol.Session, error) {
	a.mu.Lock()
	if a.stopped || a.client == nil {
		a.mu.Unlock()
		return protocol.Session{}, fmt.Errorf("omp: adapter not running")
	}
	a.mu.Unlock()

	if _, err := a.call(ctx, newCommand("new_session")); err != nil {
		return protocol.Session{}, err
	}
	state, err := a.getState(ctx)
	if err != nil {
		return protocol.Session{}, err
	}
	a.mu.Lock()
	a.sessionID = state.SessionID
	a.streaming = state.IsStreaming
	a.mu.Unlock()
	a.tr.reset()

	if title != "" {
		if _, err := a.call(ctx, newCommand("set_session_name").with("name", title)); err != nil {
			return protocol.Session{}, err
		}
		state.SessionName = title
	}
	sess := sessionFromState(state, a.cfg.Dir)
	a.emitSessionUpdated(sess)
	return sess, nil
}

// RenameSession sets the title. The active session renames in place; other
// sessions require a (streaming-guarded) switch round trip.
func (a *Adapter) RenameSession(ctx context.Context, sessionID, title string) error {
	if strings.TrimSpace(title) == "" {
		return fmt.Errorf("omp: session name cannot be empty")
	}
	a.mu.Lock()
	current := a.sessionID
	streaming := a.streaming
	a.mu.Unlock()

	if sessionID == current {
		if _, err := a.call(ctx, newCommand("set_session_name").with("name", title)); err != nil {
			return err
		}
		a.emitSessionUpdated(protocol.Session{ID: sessionID, Title: title, Directory: a.cfg.Dir})
		return nil
	}
	if streaming {
		return fmt.Errorf("omp: cannot rename while the agent is streaming")
	}
	prev := current
	if err := a.ensureSession(ctx, sessionID); err != nil {
		return err
	}
	if _, err := a.call(ctx, newCommand("set_session_name").with("name", title)); err != nil {
		return err
	}
	// Restore the previously active session; a failed restore leaves the
	// renamed one active, which the UI reflects via the next status event.
	_ = a.ensureSession(ctx, prev)
	a.emitSessionUpdated(protocol.Session{ID: sessionID, Title: title, Directory: a.cfg.Dir})
	return nil
}

// DeleteSession removes a session's JSONL from disk (what omp's own pickers
// do). The live session cannot be deleted.
func (a *Adapter) DeleteSession(_ context.Context, sessionID string) error {
	a.mu.Lock()
	current := a.sessionID
	a.mu.Unlock()
	if sessionID == current {
		return fmt.Errorf("omp: cannot delete the active session")
	}
	path, err := FindSessionFile(a.cfg.Dir, sessionID)
	if err != nil {
		return err
	}
	if err := osRemove(path); err != nil {
		return fmt.Errorf("omp: delete session: %w", err)
	}
	env, err := protocol.NewEnvelope(protocol.EventSessionRemoved, protocol.SessionRemoved{
		ProjectID: a.cfg.ProjectID,
		SessionID: sessionID,
	})
	if err != nil {
		return err
	}
	a.emit(env)
	return nil
}

// Messages hydrates a session's history (newest last, limit applied to the
// tail). Hydration requires the session to be active in the child: omp's
// get_messages reads the live session only.
func (a *Adapter) Messages(ctx context.Context, sessionID string, limit int) ([]agent.HydratedMessage, error) {
	a.mu.Lock()
	streaming := a.streaming
	a.mu.Unlock()
	if streaming {
		return nil, fmt.Errorf("omp: session is streaming; retry after it settles")
	}
	if err := a.ensureSession(ctx, sessionID); err != nil {
		return nil, err
	}
	resp, err := a.call(ctx, newCommand("get_messages"), 30*time.Second)
	if err != nil {
		return nil, err
	}
	if err := resp.ResponseError(); err != nil {
		return nil, err
	}
	msgs, err := parseMessagesResponse(resp.Data)
	if err != nil {
		return nil, err
	}
	return hydrateMessages(sessionID, msgs, limit)
}

// --- prompting --------------------------------------------------------------

// Prompt sends a user turn. While the agent streams, the turn is queued as
// steering (omp requires an explicit queue policy during a run; steer is the
// interrupt path, matching Circulo's abort-then-send semantics closest).
func (a *Adapter) Prompt(ctx context.Context, sessionID string, req protocol.PromptRequest) error {
	if strings.TrimSpace(req.Text) == "" {
		return fmt.Errorf("omp: empty prompt")
	}
	if err := a.ensureSession(ctx, sessionID); err != nil {
		return err
	}

	// Reasoning effort: omp models take a session-level thinking level, not
	// per-request variants; apply before the turn when the composer sent one.
	if req.Variant != "" {
		if _, err := a.call(ctx, newCommand("set_thinking_level").with("level", req.Variant)); err != nil {
			return err
		}
	}

	state, err := a.getState(ctx)
	if err != nil {
		return err
	}
	cmd := newCommand("prompt").with("message", req.Text)
	if state.IsStreaming {
		cmd = newCommand("steer").with("message", req.Text)
	}
	resp, err := a.call(ctx, cmd, 15*time.Second)
	if err != nil {
		return err
	}
	// prompt/steer ack immediately; the turn itself streams as events.
	return resp.ResponseError()
}

// Abort interrupts the current turn.
func (a *Adapter) Abort(ctx context.Context, sessionID string) error {
	if err := a.ensureSession(ctx, sessionID); err != nil {
		return err
	}
	resp, err := a.call(ctx, newCommand("abort"), 5*time.Second)
	if err != nil {
		return err
	}
	return resp.ResponseError()
}

// RunCommand invokes a slash command. omp's prompt handler routes "/name
// args" through its command dispatcher (verified live on 18.2.8: the text is
// matched against the available-commands registry), so no dedicated RPC
// exists or is needed. Text is the raw composer input; the slash prefix is
// prepended here to keep the wire contract slash-free.
func (a *Adapter) RunCommand(ctx context.Context, sessionID, name, text string) error {
	if strings.TrimSpace(name) == "" {
		return fmt.Errorf("omp: empty command name")
	}
	args := strings.TrimSpace(strings.TrimPrefix(strings.TrimSpace(text), "/"+name))
	return a.Prompt(ctx, sessionID, protocol.PromptRequest{Text: "/" + name + args})
}

// --- permissions / forms ----------------------------------------------------

// ReplyPermission is unsupported: omp's RPC mode has no permission round-trip.
// Tool approvals resolve inside omp per its tools.approvalMode configuration;
// headless prompts fail closed. Circulo's permission cards stay opencode-only
// for now.
func (a *Adapter) ReplyPermission(_ context.Context, _, _, _ string) error {
	return fmt.Errorf("omp: no permission round-trip over RPC; configure tools.approvalMode in omp settings")
}

// ReplyForm answers a pending extension UI request (select/confirm/input).
func (a *Adapter) ReplyForm(_ context.Context, _ string, formID string, answer map[string]any) error {
	a.mu.Lock()
	kind, known := a.formKinds[formID]
	a.mu.Unlock()
	if !known {
		return fmt.Errorf("omp: unknown form %s", formID)
	}
	value, _ := answer["value"].(string)

	payload := map[string]any{"id": formID}
	switch kind {
	case "confirm":
		payload["type"] = "extension_ui_response"
		if value == "" {
			payload["cancelled"] = true
		} else {
			payload["confirmed"] = value == "true"
		}
	case "select", "input":
		payload["type"] = "extension_ui_response"
		if value == "" {
			payload["cancelled"] = true
		} else {
			payload["value"] = value
		}
	default:
		return fmt.Errorf("omp: unsupported form kind %q", kind)
	}
	a.mu.Lock()
	delete(a.formKinds, formID)
	a.mu.Unlock()
	return a.client.send(payload)
}

// --- vcs / meta -------------------------------------------------------------

// Vcs reads the project's git state directly: omp exposes no git surface over
// RPC, and the question is about the project directory, not the agent.
func (a *Adapter) Vcs(_ context.Context) protocol.ProjectVcs {
	branch := gitOut(a.cfg.Dir, "rev-parse", "--abbrev-ref", "HEAD")
	if branch == "" {
		return protocol.ProjectVcs{}
	}
	return protocol.ProjectVcs{
		IsRepo:   true,
		Provider: "git",
		Branch:   branch,
	}
}

// Branches lists the project's local git branches.
func (a *Adapter) Branches(_ context.Context) []string {
	out := gitLines(a.cfg.Dir, "branch", "--format=%(refname:short)")
	return out
}

// SetBranch is a documented no-op: omp has no instruction channel over RPC,
// so Circulo cannot pin a working branch the way the opencode adapter does
// (instruction entry). The branch picker stays advisory for omp projects.
func (a *Adapter) SetBranch(_ context.Context, _, _ string) error { return nil }

// Meta returns composer picker data. omp has no named-agent registry over RPC
// (Agents stays empty); models come from the bundled catalog.
func (a *Adapter) Meta(ctx context.Context) (protocol.Meta, error) {
	resp, err := a.call(ctx, newCommand("get_available_models"), 15*time.Second)
	if err != nil {
		return protocol.Meta{}, err
	}
	if err := resp.ResponseError(); err != nil {
		return protocol.Meta{}, err
	}
	var models modelsPayload
	if err := json.Unmarshal(resp.Data, &models); err != nil {
		return protocol.Meta{}, fmt.Errorf("omp: bad model catalog: %w", err)
	}
	state, err := a.getState(ctx)
	if err != nil {
		return protocol.Meta{}, err
	}

	meta := protocol.Meta{
		Models: make([]protocol.ModelInfo, 0, len(models.Models)),
		// Agents stays empty (omp has no named-agent registry over RPC), but
		// must be an array, not null: the TS contract says list.
		Agents:      []protocol.AgentInfo{},
		AccessModes: accessModes(),
		Access:      protocol.AccessState{Mode: accessMode(a.cfg.AccessMode), Supported: true},
	}
	// Slash commands (built-ins, skills, user commands). Best-effort: a
	// failing catalog must not sink Meta.
	if resp, err := a.call(ctx, newCommand("get_available_commands"), 15*time.Second); err == nil && resp.ResponseError() == nil {
		var payload struct {
			Commands []ompCommand `json:"commands"`
		}
		if json.Unmarshal(resp.Data, &payload) == nil {
			meta.Commands = ompCommandsToNeutral(payload.Commands)
		}
	}
	for _, m := range models.Models {
		mi := protocol.ModelInfo{
			ID:        m.ID,
			Name:      m.Name,
			Provider:  m.Provider,
			Reasoning: m.Reasoning,
		}
		if m.Thinking != nil && len(m.Thinking.Efforts) > 0 {
			mi.Variants = append([]string(nil), m.Thinking.Efforts...)
			mi.Reasoning = true
		}
		meta.Models = append(meta.Models, mi)
	}
	if state.Model != nil {
		meta.DefaultProvider = state.Model.Provider
		meta.DefaultModel = state.Model.ID
	}
	return meta, nil
}

// --- internals --------------------------------------------------------------

func (a *Adapter) call(ctx context.Context, cmd command, opts ...time.Duration) (*responseFrame, error) {
	timeout := 10 * time.Second
	if len(opts) > 0 {
		timeout = opts[0]
	}
	a.mu.Lock()
	client := a.client
	a.mu.Unlock()
	if client == nil {
		return nil, fmt.Errorf("omp: adapter not running")
	}
	resp, err := client.call(ctx, cmd, timeout)
	if err != nil {
		return nil, err
	}
	return resp, nil
}

func (a *Adapter) getState(ctx context.Context) (*sessionState, error) {
	resp, err := a.call(ctx, newCommand("get_state"))
	if err != nil {
		return nil, err
	}
	if err := resp.ResponseError(); err != nil {
		return nil, err
	}
	var state sessionState
	if err := json.Unmarshal(resp.Data, &state); err != nil {
		return nil, fmt.Errorf("omp: bad get_state data: %w", err)
	}
	a.mu.Lock()
	a.streaming = state.IsStreaming
	sessionID := a.sessionID
	a.mu.Unlock()
	a.emitContext(sessionID, state.ContextUsage)
	return &state, nil
}

// emitContext translates omp's contextUsage block into the neutral
// context.updated envelope; skipped when the session isn't adopted yet or
// omp reported nothing (empty session).
func (a *Adapter) emitContext(sessionID string, cu *struct {
	Tokens        int64   `json:"tokens"`
	ContextWindow int64   `json:"contextWindow"`
	Percent       float64 `json:"percent"`
}) {
	if sessionID == "" || cu == nil {
		return
	}
	env, err := protocol.NewEnvelope(protocol.EventContextUpdated, protocol.ContextUpdated{
		Usage: protocol.ContextUsage{
			ProjectID: a.cfg.ProjectID,
			SessionID: sessionID,
			Used:      cu.Tokens,
			Window:    cu.ContextWindow,
		},
	})
	if err != nil {
		return
	}
	a.emit(env)
}

// ensureSession makes sessionID the child's active session. omp is
// single-session per process: prompt/abort/messages all target the active
// session, so cross-session requests switch first.
func (a *Adapter) ensureSession(ctx context.Context, sessionID string) error {
	a.mu.Lock()
	if a.sessionID == sessionID {
		a.mu.Unlock()
		return nil
	}
	if a.streaming {
		a.mu.Unlock()
		return fmt.Errorf("omp: agent is streaming; abort first")
	}
	a.mu.Unlock()

	path, err := FindSessionFile(a.cfg.Dir, sessionID)
	if err != nil {
		return err
	}
	if _, err := a.call(ctx, newCommand("switch_session").with("sessionPath", path)); err != nil {
		return err
	}
	state, err := a.getState(ctx)
	if err != nil {
		return err
	}
	if state.SessionID != sessionID {
		return fmt.Errorf("omp: switch landed on session %s, want %s", state.SessionID, sessionID)
	}
	a.mu.Lock()
	a.sessionID = state.SessionID
	a.mu.Unlock()
	a.tr.reset()
	return nil
}

func (a *Adapter) emitSessionUpdated(sess protocol.Session) {
	if sess.Directory == "" {
		sess.Directory = a.cfg.Dir
	}
	env, err := protocol.NewEnvelope(protocol.EventSessionUpdated, protocol.SessionUpdated{
		ProjectID: a.cfg.ProjectID,
		Session:   sess,
	})
	if err != nil {
		return
	}
	a.emit(env)
}

func sessionFromState(s *sessionState, dir string) protocol.Session {
	now := time.Now().UnixMilli()
	return protocol.Session{
		ID:          s.SessionID,
		Title:       s.SessionName,
		Directory:   dir,
		TimeCreated: now,
		TimeUpdated: now,
	}
}

// gitOut runs a git command in dir and returns trimmed stdout ("" on any
// failure — git questions are best-effort by contract).
func gitOut(dir string, args ...string) string {
	cmd := exec.Command("git", args...)
	cmd.Dir = dir
	var out bytes.Buffer
	cmd.Stdout = &out
	if err := cmd.Run(); err != nil {
		return ""
	}
	return strings.TrimSpace(out.String())
}

// gitLines runs a git command returning one line per output line.
func gitLines(dir string, args ...string) []string {
	raw := gitOut(dir, args...)
	if raw == "" {
		return nil
	}
	lines := strings.Split(raw, "\n")
	out := make([]string, 0, len(lines))
	for _, l := range lines {
		if l = strings.TrimSpace(l); l != "" {
			out = append(out, l)
		}
	}
	return out
}
