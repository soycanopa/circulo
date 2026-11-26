# Circulo — Implementation Plan (v1.0)

- **Status:** Draft v1.0 (2026-11-21)
- **Related:** [PRD](prd.md) · [TRD](trd.md)
- **Goal:** Reach Waku initial feature parity on Rust + GPUI without copying code.

## Phase 0 — Foundation
- Finalize these docs + AGENTS.md
- Repo scaffold with Cargo workspace + GPUI
- Define crate boundaries: ui, core, protocol, providers, persistence, git (puzzle pieces)
- Set up GitHub workflow (branch protection optional; PRs only for external collaborators)

## Phase 1 — Core + first provider
Order matters: prove the pipe before adding a real provider to it.

1. [x] Neutral protocol crate: versioned messages, parts, WebSocket frames, forward-compatible decode, tests.
2. [x] Driver trait and a demo provider that emits a canned turn.
3. [x] Core: session registry and turn orchestration behind the Driver trait.
4. [x] Daemon process: loopback WebSocket, token handshake, request loop, announced URL.
5. [x] Client crate: handshake, requests, streamed messages. Integration test against a real daemon.
6. [x] Wire the desktop: the app supervises the daemon, streams parts into the transcript, and the daemon exits with the app.
7. [x] OpenCode adapter behind the same Driver trait (ACP over stdio).
8. [x] Local persistence (SQLite), owned by the core: sessions, turns and parts in `~/.circulo/circulo.db` (`CIRCULO_HOME` overrides), the app reopens the newest session with its history. Settings followed as the `settings` table (schema 3); the attachment store still follows.
9. [x] Session continuity: one agent process per Circulo session, the ACP session id stored in SQLite, `session/load` on a daemon restart, mid-turn option switches land at the turn's end, clean daemon shutdown with no orphaned agent processes.

## Phase 2 — Shared control surface
- [x] Model / reasoning effort discovery and switching (`session/new` returns `configOptions`; `session/set_config_option`)
- [x] Message queue and steering: `Prompt` while a turn runs queues FIFO; `Steer` puts the text at the front and aborts the running turn; the queue travels on the wire and the window can remove an item
- [x] Abort: `CancelTurn` cancels the turn's cards, the driver sends ACP `session/cancel`, and the turn ends as `aborted`

## Phase 3 — Second provider + Git
- [x] Projects: the composer's project chip lists canonical folders, `New project…` uses the native picker, `No project` is a row, and a session's agent runs in its project's `cwd`; the sidebar row keeps title / project / time
- [x] Git checkpoints captured at turn start and end, stored under `refs/circulo/...`; the composer footer shows the real branch when the project is a repository
- [x] Conversation-aware rewind: restore a checkpoint, trim the transcript, drop the provider session, and confirm the destructive parts in a card
- Claude Code or Codex adapter

## Phase 4 — UI polish
- [x] Sidebar from the database: sessions grouped by day (Today / Yesterday / This Month / month), title, project and relative time, the open session marked; a row opens its history, `New Task` starts a session
- [x] Beautiful markdown transcript: headings, lists, tables, fenced code, quotes, footnotes, weighted columns, cached parse
- [x] Permission cards, and question cards from ACP elicitation
- [x] Typeable composer: Enter sends, Shift+Enter breaks the line, prompts travel mid-turn
- [x] Tool rows keyed by the provider's call id; the transcript follows the stream with a jump-to-latest button
- Status banners
- Keyboard shortcuts

## Phase 5 — Hardening & packaging
- Performance pass on long transcripts
- Live smoke tests against real CLIs
- Cargo packaging (.app / installers for macOS/Linux/Windows)
- E2E checklist

## Phase 6 — v1.0 exit
- Demonstrate parity items from PRD
- Documentation update
- Tag v1.0

## Post v1.0
- Additional providers
- Mobile (evaluate options: GPUI mobile if available, or separate UI layer sharing the core protocol — e.g. SwiftUI/Kotlin or a web view)
- Remote access
- Right-side context panel (files, diffs, terminal)
- Computer Use (evaluate)

## E2E checklist (repeat before release)
1. Add project
2. New session with two different providers
3. Stream text + tools
4. Switch model mid-session
5. Steer or queue while working
6. Permission card flow
7. Create checkpoint and rewind
8. Quit cleanly (no orphaned processes)
9. Reopen and hydrate history