## Why

The app cannot talk to Circulo yet: there is no local HTTP/SSE daemon that exposes health, projects, sessions, or generation. Persistence and the fake adapter exist; they need a process boundary.

## What Changes

- Run `circulo-daemon` as a localhost HTTP server (bind **127.0.0.1 only**).
- Expose `/v1/health`, project/session/message APIs, preferences, and SSE `/v1/sessions/{id}/events`.
- POST a user message runs the **fake** adapter, persists the turn, and emits typed protocol events.
- Errors use `ApiError` codes (not raw OS strings).
- **TLS/HTTPS is not implemented in this change.** Certificates remain an open TRD decision. The API is HTTP on loopback only.

## Capabilities

### New Capabilities

- `local-daemon-api`: Localhost daemon HTTP + SSE contract used by the Circulo app.

### Modified Capabilities

- (none)

## Non-goals

- No TLS/HTTPS (open; not silently decided as “never”).
- No OpenCode adapter, no GPUI, no cancel endpoint (P1).
- No bind on `0.0.0.0`.
- No worktree switching.

## Impact

- Crate `circulo-daemon` becomes a real server (axum + tokio).
- Small persist helpers if PATCH/archive session need them.
- Default listen: `127.0.0.1:7432` (override with `CIRCULO_DAEMON_ADDR` if loopback).

## Open questions (not resolved here)

- Local TLS/certs (TRD-API-02).
- How the app spawns/reuses the daemon (app-shell change).
