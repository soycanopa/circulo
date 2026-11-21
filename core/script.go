// Package core owns session state for the core process.
// It never imports the UI or MyGo.
package core

import "github.com/soycanopa/circulo/protocol"

// LiveID is the message the prototype streams token by token.
// Earlier messages are history, each delivered whole.
const LiveID = "live"

// Script is the prototype session: a scrollable history, then one streaming reply.
func Script() []protocol.Envelope {
	history := []struct{ id, role, text string }{
		{"h1", "user", "Add a project and show me the session."},
		{"h2", "assistant", "Project **graphite** is on the list. Session alpha is idle."},
		{"h3", "user", "Summarize the layout."},
		{"h4", "assistant", "Sidebar, transcript, composer. The transcript stays pinned to the latest turn."},
		{"h5", "user", "What belongs in the core process?"},
		{"h6", "assistant", "Projects, sessions, providers, Git, and storage. The window only renders."},
		{"h7", "user", "Queue a follow-up."},
		{"h8", "assistant", "Queued. The running turn keeps its place until it finishes."},
		{"h9", "user", "Show a short table of states."},
		{"h10", "assistant", "| State | Surface |\n| --- | --- |\n| Starting | Banner |\n| Streaming | Live parts |\n| Idle | Composer |"},
		{"h11", "user", "Remind me where checkpoints live."},
		{"h12", "assistant", "The core writes them. Rewind restores the worktree and the conversation around that turn."},
	}
	out := make([]protocol.Envelope, 0, len(history)*2+len(liveChunks())+2)
	var seq int64
	next := func(e protocol.Envelope) protocol.Envelope {
		seq++
		e.V = protocol.Version
		e.Seq = seq
		return e
	}
	out = append(out, next(protocol.Envelope{Type: protocol.TypeHello}))
	for _, m := range history {
		out = append(out,
			next(protocol.Envelope{Type: protocol.TypeDelta, ID: m.id, Role: m.role, Text: m.text}),
			next(protocol.Envelope{Type: protocol.TypeDone, ID: m.id}),
		)
	}
	for _, chunk := range liveChunks() {
		out = append(out, next(protocol.Envelope{Type: protocol.TypeDelta, ID: LiveID, Role: "assistant", Text: chunk}))
	}
	out = append(out, next(protocol.Envelope{Type: protocol.TypeDone, ID: LiveID}))
	return out
}

func liveChunks() []string {
	const reply = "# Streaming reply\n\n" +
		"This turn is still open. Tokens arrive from the core process, and the window paints them with native markdown.\n\n" +
		"- Pin to the latest line\n" +
		"- Keep the selection while text grows\n" +
		"- Leave finished blocks alone\n\n" +
		"```go\nfunc main() {\n\tprintln(\"circulo\")\n}\n```\n\n" +
		"A finished reply switches from the stream to a settled document, with the same source.\n"
	const size = 24
	var chunks []string
	for i := 0; i < len(reply); i += size {
		j := i + size
		if j > len(reply) {
			j = len(reply)
		}
		chunks = append(chunks, reply[i:j])
	}
	return chunks
}
