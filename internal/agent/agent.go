// Package agent defines the adapter contract every coding-agent backend must
// satisfy. It is deliberately tiny: lifecycle, session CRUD, prompting,
// permissions, and a read-only channel of neutral protocol events.
//
// Dependency rule (AGENTS.md): this package and its implementors must not
// import Wails, relay, or orchestrator packages. The UI consumes adapters
// exclusively through internal/relay.
package agent

import (
	"context"

	"circulogo/internal/agent/protocol"
)

// Adapter is one project's connection to an agent CLI backend.
// Implementations must:
//   - stamp every emitted event payload with the projectID given at construction;
//   - never block Events() consumers (send with select/default or buffer);
//   - be safe for concurrent method calls.
type Adapter interface {
	// Start brings the backend up. For managed mode this spawns the CLI
	// server; for attach mode it health-checks an existing one. Start returns
	// once the backend is serving (or errors with the cause).
	Start(ctx context.Context) error
	// Stop terminates the backend (managed mode) and closes the event stream.
	// Stop must be idempotent.
	Stop(ctx context.Context) error

	// Events yields neutral protocol envelopes until Stop. The channel is
	// closed by Stop.
	Events() <-chan protocol.Envelope

	// Sessions lists sessions known to the backend.
	Sessions(ctx context.Context) ([]protocol.Session, error)
	// CreateSession starts a new session, optionally titled.
	CreateSession(ctx context.Context, title string) (protocol.Session, error)
	// RenameSession sets the session title.
	RenameSession(ctx context.Context, sessionID, title string) error
	// DeleteSession removes the session and its history on the backend.
	DeleteSession(ctx context.Context, sessionID string) error

	// Messages hydrates a session's history (newest last when limit > 0).
	Messages(ctx context.Context, sessionID string, limit int) ([]HydratedMessage, error)

	// Prompt sends a user turn; content streams back over Events().
	Prompt(ctx context.Context, sessionID string, req protocol.PromptRequest) error
	// Abort interrupts the current turn.
	Abort(ctx context.Context, sessionID string) error

	// ReplyPermission answers a permission request (once|always|reject).
	ReplyPermission(ctx context.Context, sessionID, permissionID, response string) error

	// Meta returns composer picker data (agents, models).
	Meta(ctx context.Context) (protocol.Meta, error)
}

// HydratedMessage is one message with all its parts from history.
// JSON tags matter: the relay marshals this straight to the webview, which
// expects the protocol's camelCase contract.
type HydratedMessage struct {
	Info  protocol.MessageInfo `json:"info"`
	Parts []protocol.Part      `json:"parts"`
}
