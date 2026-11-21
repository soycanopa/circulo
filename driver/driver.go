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
	// KindModel means the session's model or reasoning effort changed.
	KindModel = "model.selected"
)

// Event is one normalized provider event.
type Event struct {
	Kind       string
	SessionID  string
	MessageID  string
	Text       string
	ModelID    string
	ProviderID string
	Variant    string
}

// Model is a provider model the user can select.
// Variant is the reasoning effort in use. Variants lists the efforts the model offers.
type Model struct {
	ID         string
	ProviderID string
	Name       string
	Variant    string
	Variants   []string
}

// Driver talks to one coding-agent CLI. The core owns when it is called.
// Delivery on Prompt is empty, "steer", or "queue".
type Driver interface {
	CreateSession(ctx context.Context, directory, title string) (sessionID string, err error)
	Prompt(ctx context.Context, sessionID, text, delivery string) error
	Models(ctx context.Context, directory string) ([]Model, error)
	DefaultModel(ctx context.Context, directory string) (Model, error)
	SessionModel(ctx context.Context, sessionID string) (Model, error)
	SetModel(ctx context.Context, sessionID string, model Model) error
	Abort(ctx context.Context, sessionID string) error
	Watch(ctx context.Context, fn func(Event)) error
}
