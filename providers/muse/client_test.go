package muse

import (
	"bufio"
	"context"
	"encoding/json"
	"io"
	"strings"
	"testing"

	"github.com/soycanopa/circulo/driver"
)

func TestSessionPromptAndDelta(t *testing.T) {
	cr, cw := io.Pipe()
	pr, pw := io.Pipe()
	go fakeMuse(t, pr, cw)
	c := attach(cr, pw)
	ctx := context.Background()
	if err := c.handshake(ctx); err != nil {
		t.Fatal(err)
	}
	id, err := c.CreateSession(ctx, "/work", "work")
	if err != nil || id != "ses_muse" {
		t.Fatalf("session %q %v", id, err)
	}
	models, err := c.Models(ctx, "/work")
	if err != nil || len(models) != 1 || models[0].ID != "muse-free" || len(models[0].Variants) != 2 {
		t.Fatalf("models %#v %v", models, err)
	}
	if err := c.SetModel(ctx, id, driver.Model{ID: "muse-free", ProviderID: "meta", Variant: "low"}); err != nil {
		t.Fatal(err)
	}
	got := make(chan driver.Event, 4)
	go c.Watch(ctx, func(ev driver.Event) { got <- ev })
	if err := c.Prompt(ctx, id, "hello", "steer"); err != nil {
		t.Fatal(err)
	}
	ev := <-got
	if ev.Kind != driver.KindDelta || ev.Text != "Hi" || ev.SessionID != "ses_muse" {
		t.Fatalf("%#v", ev)
	}
	if err := c.Abort(ctx, id); err != nil {
		t.Fatal(err)
	}
}

func fakeMuse(t *testing.T, r io.Reader, w io.Writer) {
	t.Helper()
	sc := bufio.NewScanner(r)
	bw := bufio.NewWriter(w)
	reply := func(v any) {
		if err := json.NewEncoder(bw).Encode(v); err != nil {
			t.Error(err)
		}
		bw.Flush()
	}
	for sc.Scan() {
		var req struct {
			ID     any             `json:"id"`
			Method string          `json:"method"`
			Params json.RawMessage `json:"params"`
		}
		if err := json.Unmarshal(sc.Bytes(), &req); err != nil {
			t.Error(err)
			return
		}
		switch req.Method {
		case "initialize":
			reply(map[string]any{"jsonrpc": "2.0", "id": req.ID, "result": map[string]any{}})
		case "initialized":
		case "session/start":
			if !strings.Contains(string(req.Params), `"workspaceRoot":"/work"`) || !strings.Contains(string(req.Params), `"allowAll"`) {
				t.Errorf("start %s", req.Params)
			}
			reply(map[string]any{"jsonrpc": "2.0", "id": req.ID, "result": map[string]any{"session": map[string]any{"sessionId": "ses_muse"}}})
		case "model/list":
			reply(map[string]any{"jsonrpc": "2.0", "id": req.ID, "result": map[string]any{
				"providerId": "meta",
				"models": []any{map[string]any{
					"modelId": "muse-free", "displayLabel": "Muse Free", "providerId": "meta", "isDefault": true,
					"reasoningEffortVariants": []string{"low", "high"},
				}},
			}})
		case "session/setModel":
			if !strings.Contains(string(req.Params), `"modelId":"muse-free"`) {
				t.Errorf("setModel %s", req.Params)
			}
			reply(map[string]any{"jsonrpc": "2.0", "id": req.ID, "result": map[string]any{}})
		case "session/setReasoningEffort":
			if !strings.Contains(string(req.Params), `"reasoningEffort":"low"`) {
				t.Errorf("effort %s", req.Params)
			}
			reply(map[string]any{"jsonrpc": "2.0", "id": req.ID, "result": map[string]any{}})
		case "turn/start":
			if !strings.Contains(string(req.Params), `"text":"hello"`) || !strings.Contains(string(req.Params), `"ifBusy":"steer"`) {
				t.Errorf("turn %s", req.Params)
			}
			reply(map[string]any{"jsonrpc": "2.0", "id": req.ID, "result": map[string]any{"turnId": "turn_1"}})
			reply(map[string]any{"jsonrpc": "2.0", "method": "item/delta", "params": map[string]any{
				"delta": "Hi", "itemId": "item_1", "sessionId": "ses_muse", "viewCursor": "1",
			}})
		case "turn/interrupt":
			reply(map[string]any{"jsonrpc": "2.0", "id": req.ID, "result": map[string]any{}})
		default:
			t.Errorf("unexpected %s", req.Method)
		}
	}
}
