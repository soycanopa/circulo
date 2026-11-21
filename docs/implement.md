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

1. Neutral protocol crate: versioned messages, parts, framing, forward-compatible decode, tests.
2. Driver trait and a demo provider that emits a canned turn. Nothing real is spawned yet.
3. Core: session registry and turn orchestration behind the Driver trait.
4. Daemon process: loopback TCP, token handshake, request loop, announced address.
5. Client crate: handshake, requests, streamed messages. Integration test against a real daemon.
6. Wire the desktop: spawn the daemon, stream parts into the transcript. The mock module goes away.
7. OpenCode adapter behind the same Driver trait.
8. Local persistence (SQLite), owned by the core.

## Phase 2 — Shared control surface
- Model / reasoning effort discovery and switching
- Message queue + steering
- Abort

## Phase 3 — Second provider + Git
- Claude Code or Codex adapter
- Conversation-aware Git checkpoints and rewind

## Phase 4 — UI polish
- Full sidebar (projects + sessions)
- Beautiful markdown transcript
- Permission cards
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