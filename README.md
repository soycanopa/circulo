# circuloGo

Local-first desktop orchestrator for coding-agent CLIs, starting with
[OpenCode](https://opencode.ai). One native window (Wails v3) that manages
projects, sessions and live streaming against a local `opencode serve` —
no cloud, everything bound to `127.0.0.1`.

**Status:** v0 in progress — see [docs/implement.md](docs/implement.md) for the
phase plan and what is done/outstanding.

## Stack

- **Backend:** Go 1.25+ (`internal/…`), Wails **v3.0.0-beta.16** (pinned — beta APIs shift)
- **Frontend:** React 18 + TypeScript + Vite + Tailwind v4 (shadcn/ui), zustand
- **Agent:** OpenCode **v2** (`2.0.x`, fixtures/pins recorded against 2.0.8;
  see [docs/opencode-v2-migration.md](docs/opencode-v2-migration.md))
- **Tests:** Go testing + httptest; vitest

## Getting started

Requirements: Go, Node + pnpm, [wails3 CLI v3.0.0-beta.16](https://v3.wails.io/learn/install/),
`task` (go-task), and an `opencode` binary on `PATH` for managed projects.

```sh
# app dev mode (webview + hot reload)
task dev            # runs: wails3 dev -config ./build/config.yml -port 9245

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
internal/store            atomic settings.json (project registry)
docs/                     prd · trd · ux · flow · ui · implement · remote
```

Architecture invariants and workflow rules live in
[AGENTS.md](AGENTS.md) — read it before changing anything; it is the
constitution of this repo (dependency direction, protocol-change discipline,
Wails/OpenCode traps, gated work).
