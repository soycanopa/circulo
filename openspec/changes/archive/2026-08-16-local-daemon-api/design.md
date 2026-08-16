## Context

See `proposal.md`. Persist + fake adapter exist. TRD-API-02 (HTTPS/certs) is still open.

## Goals / Non-Goals

**Goals:**

- axum server on loopback, in-process `Store` + `Arc<dyn AgentAdapter>`.
- CRUD needed for the MVP API surface (minus cancel).
- Generate via fake, map `AdapterEvent` → persist + `ProtocolEvent`.
- Integration tests against `127.0.0.1`.

**Non-Goals:**

- rustls / certificates.
- App process supervisor.
- OpenCode.

## Decisions

### 1. HTTP on 127.0.0.1, not HTTPS yet

**Why:** TRD leaves certs open. Self-signed HTTPS on localhost is extra friction for no network exposure if we bind loopback only. This change does **not** close “we will never do HTTPS”; it defers TLS.

**Alternative:** rcgen + rustls now. Rejected until the cert UX is decided.

### 2. axum 0.8 + tokio

Standard, SSE support, easy tests.

### 3. `circulo-daemon` is lib + bin

`router(state)` is tested without parsing CLI.

### 4. `std::sync::Mutex<Store>`

rusqlite `Connection` is not `Sync`. Handlers lock per request. Generate runs under the lock only for persist writes; adapter emit maps events then locks to save.

### 5. POST /messages waits for the fake turn

Fake is instant. Waiting keeps tests simple (`GET /messages` after POST). A later change can detach if a real adapter is slow.

### 6. Default addr `127.0.0.1:7432`

Override `CIRCULO_DAEMON_ADDR` only if the parsed address `is_loopback()`.

### 7. Cancel omitted

P1. Return 404 for unknown routes; no stub cancel.

## Risks / Trade-offs

- [Holding Mutex during generate] → fake is short; OpenCode later should not hold the DB lock across the network.
- [SSE clients that connect after POST miss events] → they still have `GET /messages`. Document that the UI should subscribe first when streaming live.

## Migration Plan

None.

## Open Questions

TLS still open. App spawn/reuse still open.
