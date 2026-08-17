# Design: adapter-opencode

## Context

The daemon runs `FakeAdapter` in production (`crates/circulo-daemon/src/main.rs:25`). The `AgentAdapter` trait is synchronous (`generate` emits `AdapterEvent`s through a callback) and `run_turn` in `crates/circulo-daemon/src/generate.rs` already persists incremental assistant state and re-emits protocol events, so the adapter only needs to produce normalized events. SQLite has a single migration V1 (`sessions` table has no agent binding column). `circulo-app` already uses `ureq`.

Verified OpenCode server facts (opencode.ai docs, server + SDK pages, 2026-08-16):

- `opencode serve [--port N] [--hostname H]` — defaults `127.0.0.1:4096`.
- `GET /doc` serves the OpenAPI 3.1 spec; usable as a liveness/identity probe.
- Sessions: `POST /session` (body `{ parentID?, title? }`), `GET/PATCH/DELETE /session/:id`.
- Prompting: `POST /session/:id/message` (blocks until the reply completes) and `POST /session/:id/prompt_async` (returns 204 immediately).
- Events: `GET /event` is an SSE stream (first event `server.connected`, then bus events shaped `{ type, properties }`).
- Optional basic auth via `OPENCODE_SERVER_PASSWORD` / `OPENCODE_SERVER_USERNAME` env vars.
- The prose docs do not enumerate turn event names/payloads; they must be pinned from a live `GET /doc` (task 1).

User-closed decisions (2026-08-16, previously TRD §15 blockers): the daemon launches and owns the OpenCode server on a dedicated port (no hybrid attach to a user's server on 4096), and the Circulo↔OpenCode session mapping is 1:1 persisted.

## Goals / Non-Goals

**Goals:**

- A real turn against OpenCode with zero manual server setup on the happy path.
- Keep the sync `AgentAdapter` trait and the fake-driven test suites intact.
- Isolate every OpenCode-specific shape (endpoints, event names, payloads) inside `circulo-adapter-opencode`.

**Non-Goals:**

- Async trait refactor; cancel support; model/agent selection; app-side SSE consumption; TLS app↔daemon (unchanged scope).
- Attaching to a foreign/user-started OpenCode server.

## Decisions

### D1. The adapter crate owns the server lifecycle

`circulo-adapter-opencode` gains a `server` module with a `ServerManager`. It probes the dedicated port (`CIRCULO_OPENCODE_PORT`, default 7433) with `GET /doc`; if a spec-bearing OpenCode answers, it is reused. Otherwise, if an `opencode` binary is found (env override `CIRCULO_OPENCODE_CMD`, then `PATH`, then `~/.opencode/bin`, `/opt/homebrew/bin`, `/usr/local/bin`), it spawns `opencode serve --port <port> --hostname 127.0.0.1` and polls `/doc` up to a startup timeout (10 s, 250 ms interval). The child handle is kept and killed on drop. The spawn strips `OPENCODE_SERVER_PASSWORD`/`OPENCODE_SERVER_USERNAME` from the child env so the daemon-owned loopback server is passwordless.

- Liveness probe requires a 200 response from `/doc`; anything else answering on the port is treated as "port occupied by a non-OpenCode process" → typed start-failure error, never spoken to.
- Alternative considered: lifecycle in the daemon — rejected; it would leak provider specifics outside the adapter and grow the daemon into a god module.

### D2. `probe()` ensures the server; first turn pays the startup cost

`probe()` runs probe→spawn→poll and maps to `AdapterHealth`: binary missing → `Missing`; port occupied or never healthy → `Error`; healthy → `Available`. `/v1/health` therefore reflects real OpenCode state and bootstraps the server at app startup (FLOWS §2). `generate()` re-ensures cheaply (one probe) before every turn.

### D3. Turn flow: subscribe first, then `prompt_async`

Inside `generate()`:

1. Ensure server (D2). On failure return `AdapterError::Unavailable`.
2. Resolve the binding: use `request.agent_session_id` if present; else `POST /session` and emit `SessionBound { agent_session_id }` as the first event so the daemon can persist it before any text streams.
3. Open `GET /event` (SSE) **before** sending the prompt so early events are not lost.
4. `POST /session/{id}/prompt_async` with `{ parts: [{ type: "text", text }] }`; expect 204.
5. Read the SSE stream, filtering events to our agent session id, until the turn terminus observed in the fixtures: `session.idle` (success) or `session.error` / a `message.updated` carrying `info.error` (failure — error wins over idle); then stop reading.
6. A bounded turn timeout (default 120 s, env-tunable) ends the turn as `Failed`.

- Alternative considered: blocking `POST /session/:id/message` (simpler, one request) — rejected: no incremental output to map into `TextDelta`s, which the spec requires.
- Text arrives as part snapshots in OpenCode, not necessarily token deltas. The adapter keeps the last snapshot text per part and emits the suffix as `TextDelta`, so the daemon's append logic (`apply_event`) is unchanged.

### D4. Event mapping is fixture-driven and fail-open for unknown types

One `mapping` module translates pinned event/part shapes into `AdapterEvent`s (full table in `tests/fixtures/EVENTS.md`): `message.part.delta` → `TextDelta` (true deltas exist), text snapshots in `message.part.updated` → `TextDelta` with the suffix beyond the last emitted offset for that part (deltas and snapshots overlap), tool parts → `ToolCallStarted`/`ToolCallUpdated` (with `ToolOutput` text/error; diffs stay text and the renderer detects them), `todo.updated` → `TaskList`, `session.idle`/`session.error` → `Completed`/`Failed`. A turn spans multiple assistant step-messages whose text concatenates in order. Unknown `type` values are skipped with a log line to stderr and never fail the turn — the UI fallback for unknown parts already exists. Fixtures captured from a live server are committed under `crates/circulo-adapter-opencode/tests/fixtures/` and drive the mapping tests; if a future OpenCode version renames events, mapping fails closed with a typed error instead of hanging.

### D5. Binding flows through the trait, persisted write-once by the daemon

- `GenerateRequest` gains `agent_session_id: Option<String>` (the fake ignores it).
- `AdapterEvent` gains `SessionBound { agent_session_id: String }`.
- `run_turn` reads the stored binding before `generate` and persists `SessionBound` immediately (before the first text delta), so a crash mid-turn still leaves the binding durable.
- `circulo-persist` migration V2: `ALTER TABLE sessions ADD COLUMN opencode_session_id TEXT` (nullable). New store methods: `opencode_session_id(session_id)` and `bind_opencode_session(session_id, id)` which rejects storing a *different* id over an existing one (new `PersistError` variant). Storing the same id again is a no-op.
- Alternative considered: `generate` returning a `TurnSummary` — rejected; it reshapes the trait more than one additive event.

### D6. Typed error reasons; user copy lives in the locale catalog

`AdapterError` gains a stable machine reason (enum: `BinaryMissing`, `StartFailed`, `PortOccupied`, `Unauthorized`, `StreamFailed`, `Timeout`, `ProviderFailed`, `Internal` — `ProviderFailed` added during implementation for observed `session.error` failures that are neither auth nor transport) alongside the existing message, so the adapter stays i18n-free. The daemon maps reason → locale key in `en.json` (`opencode.error.*`) when building `ApiError`/failed-message text. `Unavailable` is reserved for `BinaryMissing`/`StartFailed`/`PortOccupied`; the rest are `Failed`.

### D7. Blocking adapter off the async runtime

`run_turn` (and the health probe) move onto `tokio::task::spawn_blocking` in the HTTP handlers. The trait stays synchronous and `ureq` (sync HTTP + streaming response reads) fits it; no async bridge is introduced. The fake adapter is unaffected.

### D8. Adapter selection and test injection

`circulo-daemon` main builds the adapter from `CIRCULO_ADAPTER` (default `opencode`, `fake` for the existing test suites and dev runs). Crate boundaries: `circulo-adapter-opencode` depends only on `circulo-adapter`, `circulo-core`, and `ureq`; only `circulo-daemon` may depend on it (`check-crate-boundaries.py` keeps the app clean). Adapter integration tests run against a scripted fake OpenCode HTTP server (dev-dependency `axum`, already used in the workspace) that serves `/doc`, `/session`, `/session/:id/prompt_async`, and a scripted `/event` stream — no live OpenCode needed in CI; one `#[ignore]`d test exercises the real binary when present.

## Risks / Trade-offs

- [Event names/payloads are not in the prose docs] → Task 1 pins them from a live `/doc` into fixtures before any mapping code; mapping is isolated in one module and fails closed.
- [Cumulative part text vs deltas] → prefix-diff in the adapter (D3); fixture tests cover both snapshot and delta shapes.
- [Daemon killed with SIGKILL orphans the OpenCode child] → next daemon run probes and *reuses* the healthy leftover server; Drop-kill covers normal exits. Orphan on crash is accepted.
- [Port 7433 collision with a non-OpenCode process] → `/doc` identity check turns it into a typed `PortOccupied` error instead of protocol garbage.
- [OpenCode is cwd-sensitive (repo detection, AGENTS.md rules)] → Circulo projects are organizational, not filesystem paths, so the server is spawned with cwd `$HOME` (override `CIRCULO_OPENCODE_CWD`). Per-folder agent context is a post-MVP concern.
- [Sync trait + blocking IO] → bounded by D7 (`spawn_blocking`) and explicit timeouts on every request/stream read.

## Migration Plan

1. Land migration V2 (additive `ALTER TABLE`); older rows read as `NULL` binding — valid by D5.
2. Rollback: run the daemon with `CIRCULO_ADAPTER=fake` to restore pre-change behavior; the extra column is inert for the fake.
3. Deploy order is trivial (single machine, local daemon).

## Open Questions

- Exact SSE event names for message/part/tool updates — resolved by task 1 (fixtures), no spec or task-breakdown impact.
- Whether OpenCode needs a repo cwd for good default behavior — deferred; env override covers experimentation without code changes.
