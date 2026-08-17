# adapter-opencode

## Why

The MVP currently runs on a deterministic `FakeAdapter`: every "conversation" is a scripted turn, so Circulo cannot yet do its one job — a real chat with an agent. PRD-AGT-01 makes OpenCode the only provider of the MVP, PRD-AGT-05 forbids making a non-technical user launch a server by hand, and PRD-AGT-02 requires comprehensible failures instead of stack traces. This change puts a real OpenCode adapter behind the existing `AgentAdapter` trait and closes the two TRD §15 blockers that gated it (server discovery/launch and session identity mapping).

## What Changes

- Implement `circulo-adapter-opencode` (today a 3-line stub) as the real provider: an HTTP + SSE client for `opencode serve` that maps OpenCode turn events into the normalized `AdapterEvent` stream.
- The daemon gains OpenCode server lifecycle management: it probes Circulo's dedicated port, spawns `opencode serve --port 7433 --hostname 127.0.0.1` when the `opencode` binary is available, reuses its own server while healthy, and degrades to an "OpenCode unavailable" state otherwise — never faking a stream (FLOWS §13).
- Sessions map 1:1 to an OpenCode session: the mapping is created lazily on the first send, persisted as a nullable write-once `opencode_session_id` (SQLite migration V2), and reused across daemon restarts so OpenCode keeps the native conversation context.
- `/v1/health` reports the real OpenCode availability through the adapter probe.
- Typed, human errors for the failure modes a user can hit: OpenCode binary missing, server failing to start, transport/stream failure mid-turn. Copy lives in the `en` locale catalog.
- Production wiring: the daemon runs `OpenCodeAdapter` instead of `FakeAdapter`; the fake remains available for tests via `CIRCULO_ADAPTER=fake`.

## Capabilities

### New Capabilities

- `opencode-adapter`: how the daemon obtains and manages an OpenCode server, maps each Circulo session to one OpenCode session (lazily, persisted), streams normalized turn events, and surfaces typed human errors.

### Modified Capabilities

- `local-persistence`: adds the requirement to persist an optional, write-once `opencode_session_id` per session so agent-side continuity survives restarts.

## Impact

- Code: `crates/circulo-adapter-opencode` (real implementation), `crates/circulo-daemon` (server lifecycle, turn wiring, `main`), `crates/circulo-persist` (migration V2 + store accessors), `crates/circulo-adapter`/`circulo-core` (types to carry the agent session binding through a turn), `crates/circulo-i18n` (`en.json` error strings).
- Dependencies: `ureq` added to `circulo-adapter-opencode` (sync HTTP + SSE response streaming; already a workspace dependency via `circulo-app`). Approved with this change.
- External contract verified against the official docs (opencode.ai server + SDK pages): `POST /session`, `POST /session/:id/prompt_async`, `GET /event` SSE. The exact event names are not enumerated in the prose docs and will be pinned from the live OpenAPI spec (`GET /doc`) as committed fixtures during implementation.
- Decisions closed by the user on 2026-08-16 (previously TRD §15 blockers): the daemon launches and owns the OpenCode server on a dedicated port (no hybrid attach to a user's 4096 server), and the session mapping is 1:1 persisted.

## Non-goals

- Model or agent selection (default agent/model; no selector in this change).
- Canceling a generation in flight (stays P1 — PRD-CHT-09).
- Interactive questions / permission responses (`QuestionCard` out of MVP).
- App-side live SSE consumption and app↔daemon TLS (belong to later hardening).
- Any provider other than OpenCode.

## Open questions

None blocking. Assumptions recorded in `design.md`: dedicated port 7433 next to the daemon's 7432, env overrides for test injection, write-once mapping semantics, default model/agent.
