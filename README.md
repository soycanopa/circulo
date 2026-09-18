# circuloGo

Local-first desktop orchestrator for coding-agent CLIs, starting with
[OpenCode](https://opencode.ai). One native window (Wails v3) that manages
projects, sessions and live streaming against a local `opencode serve` —
no cloud, everything bound to `127.0.0.1`.

Canonical repository: **[soycanopa/circulo](https://github.com/soycanopa/circulo)** —
the project that lived in this repo before circuloGo is preserved under `legacy/*`.

**Status:** v0 in progress. The adapter speaks OpenCode **v2 only** (verified against
2.0.8 — [docs/opencode-v2-migration.md](docs/opencode-v2-migration.md)); what gates
v0 is the owner E2E pass
([docs/opencode-v2-phase6-e2e.md](docs/opencode-v2-phase6-e2e.md)) and the phase
plan in [docs/implement.md](docs/implement.md).

## What's in the window

- **Sessions first.** Projects (folders) hold sessions; a new session picks project +
  branch up front and materializes on the first message.
- **Live streaming chat.** Reasoning, answer and tool calls as separate streaming
  parts; word-by-word text reveal; collapsible thinking trace grouped by work type
  (Reasoning / Search / Coding / Tools).
- **Agent output as UI, not text.** Permission and question cards (one question at a
  time, auto-advancing), code panel with line numbers, unified diff with old/new
  gutters, dotted flowchart canvas (`circulogo-flow` blocks).
- **PTY terminals.** One tabbed terminal surface per project, below the chat
  (`internal/term`, creack/pty).
- **Local only.** The UI never talks to the agent directly: neutral wire protocol
  (`internal/agent/protocol`) ⇄ orchestrator ⇄ `/agent` relay. OpenCode is spoken
  only inside `internal/opencode`.

## Stack

- **Backend:** Go 1.25+ (`internal/…`), Wails **v3.0.0-beta.16** (pinned — beta APIs shift)
- **Frontend:** React 18 + TypeScript + Vite + Tailwind v4 (shadcn/ui), zustand
- **Agent:** OpenCode **v2** (`2.0.x`, fixtures/pins recorded against 2.0.8)
- **Tests:** Go testing + httptest; vitest

## Getting started

Requirements: Go, Node + pnpm, [wails3 CLI v3.0.0-beta.16](https://v3.wails.io/learn/install/),
`task` (go-task), and an `opencode` binary on `PATH` for managed projects
(`CIRCULOGO_OPENCODE_BIN` overrides it — required for v2 side-by-side installs).

```sh
# app dev mode (webview + hot reload)
task dev            # runs: wails3 dev -config ./build/config.yml -port 9245

# without wails3/task: build the embedded UI, then compile the app binary
pnpm -C frontend build && go build -o build/bin/circulogo .

# CI-lite — must be green before every push
go test ./...
pnpm --dir frontend test

# package a .app (Phase 7, pending)
task package
```

Optional: `CIRCULOGO_DEBUG_ADDR=127.0.0.1:9246` serves the embedded UI next to
the `/agent` relay on a loopback listener for curl/script E2E without the
webview.

## Layout

```
frontend/src/lib/agent/   protocol.ts (TS mirror of the wire contract), api/sse/store/reducer
internal/agent/protocol   the neutral wire contract — Go side of protocol.ts
internal/opencode         the only package allowed to speak OpenCode (client, SSE, translate)
internal/orchestrator     projects → adapters, event fan-out, persistence
internal/relay            HTTP ⇄ orchestrator translation, mounted at /agent — no business logic
internal/term             PTY terminals per project (creack/pty), I/O bridged by the relay
internal/store            atomic settings.json (project registry)
docs/                     prd · trd · ux · flow · ui · implement · remote · opencode-v2 migration + handoff
```

Architecture invariants and workflow rules live in
[AGENTS.md](AGENTS.md) — read it before changing anything; it is the
constitution of this repo (dependency direction, protocol-change discipline,
Wails/OpenCode traps, gated work).
