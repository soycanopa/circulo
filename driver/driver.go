// Package driver is the in-process contract between the core and a provider.
// The wire protocol stays in package protocol. Providers do not import the core.
package driver

import "context"

const (
	// KindDelta is a chunk of assistant text.
	KindDelta = "text.delta"
	// KindEnded is the full assistant text for one part, which may repeat what deltas already sent.
	KindEnded = "text.ended"
	// KindDone means the provider finished the turn without a failure.
	KindDone = "exec.done"
	// KindFailed means the provider finished the turn with an error in Text.
	KindFailed = "exec.failed"
)

// Event is one normalized provider event.
type Event struct {
	Kind      string
	SessionID string
	MessageID string
	Text      string
}

// Driver talks to one coding-agent CLI. The core owns when it is called.
type Driver interface {
	CreateSession(ctx context.Context, directory, title string) (sessionID string, err error)
	Prompt(ctx context.Context, sessionID, text string) error
	Watch(ctx context.Context, fn func(Event)) error
}
