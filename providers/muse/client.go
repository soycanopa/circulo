// Package muse speaks Muse Session Protocol v1 over stdio.
// It does not import the core or the UI.
//
// `muse serve` reads and writes one JSON-RPC 2.0 object per line.
// Session creation is session/start, a prompt is turn/start, and stop is turn/interrupt.
// Model variants are the reasoning-effort tiers.
package muse

import (
	"bufio"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"os/exec"
	"strconv"
	"strings"
	"sync"

	"github.com/google/uuid"

	"github.com/soycanopa/circulo/driver"
)

// Client is one `muse serve` process.
type Client struct {
	mu      sync.Mutex
	w       *bufio.Writer
	pending map[string]chan rpcResult
	notes   chan note
	seq     int
	cmd     *exec.Cmd
	models  map[string]driver.Model
}

type rpcResult struct {
	raw json.RawMessage
	err error
}

type note struct {
	method string
	params json.RawMessage
}

// Start runs `bin serve` and completes the MSP handshake.
func Start(ctx context.Context, bin string) (*Client, error) {
	if bin == "" {
		bin = "muse"
	}
	cmd := exec.CommandContext(ctx, bin, "serve")
	stdin, err := cmd.StdinPipe()
	if err != nil {
		return nil, err
	}
	stdout, err := cmd.StdoutPipe()
	if err != nil {
		return nil, err
	}
	cmd.Stderr = os.Stderr
	if err := cmd.Start(); err != nil {
		return nil, fmt.Errorf("start muse: %w", err)
	}
	c := &Client{
		w:       bufio.NewWriter(stdin),
		pending: map[string]chan rpcResult{},
		notes:   make(chan note, 256),
		cmd:     cmd,
		models:  map[string]driver.Model{},
	}
	go c.read(stdout)
	if err := c.handshake(ctx); err != nil {
		_ = cmd.Process.Kill()
		return nil, err
	}
	return c, nil
}

// attach speaks MSP on an existing pipe. Tests use it instead of a real process.
func attach(r io.Reader, w io.Writer) *Client {
	c := &Client{
		w:       bufio.NewWriter(w),
		pending: map[string]chan rpcResult{},
		notes:   make(chan note, 256),
		models:  map[string]driver.Model{},
	}
	go c.read(r)
	return c
}

// Stop kills the serve process.
func (c *Client) Stop() {
	if c.cmd != nil && c.cmd.Process != nil {
		_ = c.cmd.Process.Kill()
		_ = c.cmd.Wait()
	}
}

// CreateSession starts a session rooted at directory.
// Approval is allowAll so a turn is not stuck waiting for a permission card.
func (c *Client) CreateSession(ctx context.Context, directory, title string) (string, error) {
	var out struct {
		Session struct {
			SessionID string `json:"sessionId"`
		} `json:"session"`
	}
	err := c.call(ctx, "session/start", map[string]any{
		"commandId":     newID(),
		"workspaceRoot": directory,
		"approvalMode":  "allowAll",
	}, &out)
	if err != nil {
		return "", err
	}
	if out.Session.SessionID == "" {
		return "", fmt.Errorf("muse: session/start returned no id")
	}
	_ = title
	return out.Session.SessionID, nil
}

// Prompt submits text. delivery is empty, steer, or queue.
func (c *Client) Prompt(ctx context.Context, sessionID, text, delivery string) error {
	params := map[string]any{
		"commandId": newID(),
		"sessionId": sessionID,
		"input":     []map[string]string{{"type": "text", "text": text}},
	}
	if delivery == "steer" || delivery == "queue" {
		params["ifBusy"] = delivery
	}
	return c.call(ctx, "turn/start", params, nil)
}

// Models lists the host catalog. directory is unused: Muse serves one catalog per process.
func (c *Client) Models(ctx context.Context, directory string) ([]driver.Model, error) {
	models, _, err := c.catalog(ctx, directory)
	return models, err
}

// DefaultModel is the catalog row marked isDefault, or the first row.
func (c *Client) DefaultModel(ctx context.Context, directory string) (driver.Model, error) {
	models, def, err := c.catalog(ctx, directory)
	if err != nil {
		return driver.Model{}, err
	}
	for _, m := range models {
		if m.ID == def {
			return m, nil
		}
	}
	if len(models) == 0 {
		return driver.Model{}, nil
	}
	return models[0], nil
}

func (c *Client) catalog(ctx context.Context, _ string) ([]driver.Model, string, error) {
	var out struct {
		ProviderID string `json:"providerId"`
		Models     []struct {
			ModelID      string          `json:"modelId"`
			DisplayLabel string          `json:"displayLabel"`
			ProviderID   string          `json:"providerId"`
			IsDefault    bool            `json:"isDefault"`
			Variants     json.RawMessage `json:"reasoningEffortVariants"`
		} `json:"models"`
	}
	if err := c.call(ctx, "model/list", map[string]any{}, &out); err != nil {
		return nil, "", err
	}
	var models []driver.Model
	var def string
	for _, m := range out.Models {
		provider := m.ProviderID
		if provider == "" {
			provider = out.ProviderID
		}
		item := driver.Model{ID: m.ModelID, ProviderID: provider, Name: m.DisplayLabel, Variants: effortList(m.Variants)}
		models = append(models, item)
		if m.IsDefault {
			def = m.ModelID
		}
	}
	return models, def, nil
}

// SessionModel reads the model stored on the session.
func (c *Client) SessionModel(ctx context.Context, sessionID string) (driver.Model, error) {
	var out struct {
		Session struct {
			ModelID    string `json:"modelId"`
			ProviderID string `json:"providerId"`
		} `json:"session"`
	}
	if err := c.call(ctx, "session/read", map[string]any{"sessionId": sessionID}, &out); err != nil {
		return driver.Model{}, err
	}
	return driver.Model{ID: out.Session.ModelID, ProviderID: out.Session.ProviderID}, nil
}

// SetModel switches the session model and, when Variant is set, its reasoning effort.
func (c *Client) SetModel(ctx context.Context, sessionID string, model driver.Model) error {
	if err := c.call(ctx, "session/setModel", map[string]any{
		"commandId": newID(),
		"sessionId": sessionID,
		"model": map[string]any{
			"modelId":    model.ID,
			"providerId": model.ProviderID,
		},
	}, nil); err != nil {
		return err
	}
	if model.Variant == "" {
		return nil
	}
	return c.call(ctx, "session/setReasoningEffort", map[string]any{
		"commandId":       newID(),
		"sessionId":       sessionID,
		"reasoningEffort": model.Variant,
	}, nil)
}

// Decide answers one approval with a choice id the host offered.
func (c *Client) Decide(ctx context.Context, sessionID, approvalID, requirementID, choiceID string) error {
	return c.call(ctx, "approval/decide", map[string]any{
		"commandId":     newID(),
		"sessionId":     sessionID,
		"approvalId":    approvalID,
		"requirementId": requirementID,
		"choiceId":      choiceID,
	}, nil)
}

// Abort interrupts the session's current foreground turn.
func (c *Client) Abort(ctx context.Context, sessionID string) error {
	return c.call(ctx, "turn/interrupt", map[string]any{
		"commandId": newID(),
		"sessionId": sessionID,
	}, nil)
}

// Watch forwards transcript and model notifications until ctx ends.
func (c *Client) Watch(ctx context.Context, fn func(driver.Event)) error {
	for {
		select {
		case <-ctx.Done():
			return ctx.Err()
		case n := <-c.notes:
			if ev, ok := c.translate(n); ok {
				fn(ev)
			}
		}
	}
}

func (c *Client) handshake(ctx context.Context) error {
	if err := c.call(ctx, "initialize", map[string]any{
		"clientInfo": map[string]string{"name": "circulo", "version": "0.1.0"},
	}, nil); err != nil {
		return err
	}
	return c.notify("initialized", nil)
}

func (c *Client) call(ctx context.Context, method string, params any, dest any) error {
	c.mu.Lock()
	c.seq++
	id := c.seq
	ch := make(chan rpcResult, 1)
	c.pending[strconv.Itoa(id)] = ch
	err := c.write(map[string]any{"jsonrpc": "2.0", "id": id, "method": method, "params": params})
	c.mu.Unlock()
	if err != nil {
		return err
	}
	select {
	case <-ctx.Done():
		return ctx.Err()
	case res := <-ch:
		if res.err != nil {
			return fmt.Errorf("muse: %s: %w", method, res.err)
		}
		if dest == nil || len(res.raw) == 0 || string(res.raw) == "null" {
			return nil
		}
		return json.Unmarshal(res.raw, dest)
	}
}

func (c *Client) notify(method string, params any) error {
	c.mu.Lock()
	defer c.mu.Unlock()
	msg := map[string]any{"jsonrpc": "2.0", "method": method}
	if params != nil {
		msg["params"] = params
	}
	return c.write(msg)
}

func (c *Client) write(v any) error {
	if err := json.NewEncoder(c.w).Encode(v); err != nil {
		return err
	}
	return c.w.Flush()
}

func (c *Client) read(r io.Reader) {
	sc := bufio.NewScanner(r)
	sc.Buffer(make([]byte, 0, 64*1024), 4<<20)
	for sc.Scan() {
		var frame struct {
			ID     any             `json:"id"`
			Method string          `json:"method"`
			Result json.RawMessage `json:"result"`
			Params json.RawMessage `json:"params"`
			Error  *struct {
				Message string `json:"message"`
			} `json:"error"`
		}
		if err := json.Unmarshal(sc.Bytes(), &frame); err != nil {
			continue
		}
		if frame.ID == nil {
			if frame.Method != "" {
				select {
				case c.notes <- note{method: frame.Method, params: frame.Params}:
				default:
				}
			}
			continue
		}
		key := fmt.Sprint(frame.ID)
		c.mu.Lock()
		ch := c.pending[key]
		delete(c.pending, key)
		c.mu.Unlock()
		if ch == nil {
			continue
		}
		if frame.Error != nil {
			ch <- rpcResult{err: fmt.Errorf("%s", frame.Error.Message)}
			continue
		}
		ch <- rpcResult{raw: frame.Result}
	}
}

func (c *Client) translate(n note) (driver.Event, bool) {
	switch n.method {
	case "item/delta":
		var p struct {
			Delta     string `json:"delta"`
			Field     string `json:"field"`
			ItemID    string `json:"itemId"`
			SessionID string `json:"sessionId"`
		}
		if err := json.Unmarshal(n.params, &p); err != nil {
			return driver.Event{}, false
		}
		if p.Field != "" && p.Field != "text" {
			return driver.Event{}, false
		}
		return driver.Event{Kind: driver.KindDelta, SessionID: p.SessionID, MessageID: p.ItemID, Text: p.Delta}, true
	case "item/completed":
		var p struct {
			SessionID string `json:"sessionId"`
			Item      struct {
				ItemID string `json:"itemId"`
				Kind   string `json:"kind"`
				Text   string `json:"text"`
			} `json:"item"`
		}
		if err := json.Unmarshal(n.params, &p); err != nil || p.Item.Kind != "agentMessage" {
			return driver.Event{}, false
		}
		return driver.Event{Kind: driver.KindEnded, SessionID: p.SessionID, MessageID: p.Item.ItemID, Text: p.Item.Text}, true
	case "turn/completed":
		var p struct {
			SessionID string `json:"sessionId"`
			Terminal  string `json:"terminal"`
			Reason    string `json:"reason"`
		}
		if err := json.Unmarshal(n.params, &p); err != nil {
			return driver.Event{}, false
		}
		if p.Terminal == "completed" {
			return driver.Event{Kind: driver.KindDone, SessionID: p.SessionID}, true
		}
		text := p.Reason
		if text == "" {
			text = p.Terminal
		}
		return driver.Event{Kind: driver.KindFailed, SessionID: p.SessionID, Text: text}, true
	case "session/modelChanged":
		var p struct {
			SessionID  string `json:"sessionId"`
			ModelID    string `json:"modelId"`
			ProviderID string `json:"providerId"`
		}
		if err := json.Unmarshal(n.params, &p); err != nil {
			return driver.Event{}, false
		}
		c.remember(p.SessionID, driver.Model{ID: p.ModelID, ProviderID: p.ProviderID})
		return driver.Event{Kind: driver.KindModel, SessionID: p.SessionID, ModelID: p.ModelID, ProviderID: p.ProviderID}, true
	case "approval/requested":
		var p struct {
			ApprovalID  string `json:"approvalId"`
			SessionID   string `json:"sessionId"`
			ToolName    string `json:"toolName"`
			RawArgs     string `json:"rawArgs"`
			Requirement string `json:"currentRequirementId"`
			Choices     []struct {
				ID       string `json:"choiceId"`
				Label    string `json:"label"`
				Decision string `json:"decision"`
			} `json:"availableChoices"`
		}
		if err := json.Unmarshal(n.params, &p); err != nil || p.ApprovalID == "" {
			return driver.Event{}, false
		}
		text := p.ToolName
		if p.RawArgs != "" {
			args := p.RawArgs
			if len(args) > 240 {
				args = args[:240]
			}
			text = text + "\n" + args
		}
		var lines []string
		for _, ch := range p.Choices {
			lines = append(lines, ch.ID+"\t"+strings.ReplaceAll(ch.Label, "\t", " ")+"\t"+ch.Decision)
		}
		return driver.Event{
			Kind: driver.KindPermit, SessionID: p.SessionID, Text: text, Tool: p.ToolName,
			ApprovalID: p.ApprovalID, RequirementID: p.Requirement, Choices: strings.Join(lines, "\n"),
		}, true
	case "session/reasoningEffortChanged":
		var p struct {
			SessionID       string `json:"sessionId"`
			ReasoningEffort string `json:"reasoningEffort"`
		}
		if err := json.Unmarshal(n.params, &p); err != nil {
			return driver.Event{}, false
		}
		prev := c.remember(p.SessionID, driver.Model{Variant: p.ReasoningEffort})
		if prev.ID == "" {
			return driver.Event{}, false
		}
		return driver.Event{
			Kind: driver.KindModel, SessionID: p.SessionID,
			ModelID: prev.ID, ProviderID: prev.ProviderID, Variant: p.ReasoningEffort,
		}, true
	default:
		return driver.Event{}, false
	}
}

func (c *Client) remember(sessionID string, next driver.Model) driver.Model {
	c.mu.Lock()
	defer c.mu.Unlock()
	cur := c.models[sessionID]
	if next.ID != "" {
		cur.ID = next.ID
	}
	if next.ProviderID != "" {
		cur.ProviderID = next.ProviderID
	}
	if next.Variant != "" {
		cur.Variant = next.Variant
	}
	c.models[sessionID] = cur
	return cur
}

func effortList(raw json.RawMessage) []string {
	if len(raw) == 0 || raw[0] != '[' {
		return nil
	}
	var out []string
	if err := json.Unmarshal(raw, &out); err != nil {
		return nil
	}
	return out
}

func newID() string {
	id, err := uuid.NewV7()
	if err != nil {
		return uuid.NewString()
	}
	return id.String()
}
