# Circulo — Implementation Plan (v1.0)

- **Status:** Draft v1.0 (2026-11-20)
- **Related:** [PRD](prd.md) · [TRD](trd.md)
- **Goal:** Reach Waku initial feature parity on MyGo without copying code.

## Phase 0 — Foundation
- Finalize these docs + AGENTS.md
- Decided (2026-11-20): native UI for the whole window, including the transcript. Core is its own process. WebView only for a later Browser tab, under the native view.
- Transcript prototype: `ui/session` shows history plus one live reply from `cmd/core` over the loopback protocol. `go tool mygo dev` starts both. The list follows the end; "Jump to latest" appears after scrolling away.
- Repo scaffold with MyGo: `mygo.json` points at `cmd/circulo`. `cmd/core` is the core process.
- Package boundaries: `ui/session`, `core`, `protocol`, `providers`, `persistence`, `git`. The UI and the core are separate processes. `arch/imports_test.go` fails the build if the UI imports the core, or the core imports MyGo.
- Set up GitHub workflow (branch protection, PR template if desired)

## Phase 1 — Core + first provider
- Project/session registry (core package)
- Neutral protocol package
- OpenCode adapter (independent package)
- Local persistence (SQLite package)
- Basic streaming transcript (minimal UI that talks only to protocol)

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
- MyGo desktop packaging (.app / installers)
- E2E checklist

## Phase 6 — v1.0 exit
- Demonstrate parity items from PRD
- Documentation update
- Tag v1.0

## Post v1.0
- Additional providers
- Mobile (MyGo iOS/Android)
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