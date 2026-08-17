# Tasks: adapter-opencode

## 1. Pin the OpenCode contract

- [x] 1.1 Run a local `opencode serve`, capture `GET /doc`, and commit representative SSE event samples (text part updates, tool part, todo list, assistant completion/failure) under `crates/circulo-adapter-opencode/tests/fixtures/`, with the observed event names documented in `tests/fixtures/EVENTS.md` (resolves design Open Question 1).
- [x] 1.2 Add a scripted `FakeOpenCodeServer` test helper (dev-deps `axum`/`tokio`, already workspace-familiar) serving `/doc`, `POST /session`, `POST /session/:id/prompt_async`, and a scripted `GET /event` SSE stream; test that each endpoint answers as the real server would.

## 2. Persistence: agent session binding

- [x] 2.1 Add migration V2 (`ALTER TABLE sessions ADD COLUMN opencode_session_id TEXT`) plus store methods `opencode_session_id(session_id)` and `bind_opencode_session(session_id, id)` with write-once semantics and a new `PersistError` variant; tests: store/reload roundtrip, unbound stays null, different id rejected, same id is a no-op.

## 3. Adapter contract extensions

- [x] 3.1 Extend `GenerateRequest` with `agent_session_id: Option<String>` and add `AdapterEvent::SessionBound { agent_session_id }`; keep `FakeAdapter` compiling (it ignores both); unit tests for the new variant.
- [x] 3.2 Add a stable machine `reason` (enum: BinaryMissing, StartFailed, PortOccupied, Unauthorized, StreamFailed, Timeout, Internal) to `AdapterError`, preserving `kind()`/`message()` behavior; unit tests.

## 4. Server manager

- [x] 4.1 Implement `server::ServerManager` per design D1–D2: env config (port, command, cwd), binary lookup (env → PATH → known locations), `/doc` identity probe, spawn with sanitized env, poll-until-healthy with 10 s timeout, child kill on drop; tests with the fake server for: reuse-when-healthy, missing binary → `Missing`, non-OpenCode port occupant → `PortOccupied`, command that never serves → `StartFailed`.

## 5. Turn orchestration and mapping

- [x] 5.1 Implement the ureq HTTP client module: create session (`POST /session`), `prompt_async` expecting 204, and a buffered `GET /event` stream reader with read timeouts; every call maps to the D6 typed reasons.
- [x] 5.2 Implement `OpenCodeAdapter::probe()` as ensure-server-then-report; tests for the three health outcomes against the fake server and a missing binary.
- [x] 5.3 Implement `OpenCodeAdapter::generate()` per design D3–D4: resolve or create the binding (emitting `SessionBound` first), subscribe before prompting, filter SSE events by agent session, map text snapshots to `TextDelta` via prefix-diff, tool parts to tool-call events, todos to `TaskList`, terminal completion/failure, and a bounded turn timeout; fixture-driven tests covering text turn, tool+task turn, unknown event skipped, mid-stream drop → `Failed`, timeout, unauthorized → `Failed`.

## 6. Daemon wiring

- [x] 6.1 Extend `run_turn` to load the stored binding into `GenerateRequest` and persist `SessionBound` immediately (write-once) before any text delta; unit test with a fake adapter that emits `SessionBound`.
- [x] 6.2 Move `run_turn` and the health probe onto `tokio::task::spawn_blocking` in the HTTP handlers (design D7).
- [x] 6.3 Wire `main.rs` adapter selection: `CIRCULO_ADAPTER` env, default `opencode`, opt-in `fake`.
- [x] 6.4 Add `opencode.error.*` keys to `locales/en.json` and map every error reason to catalog copy where the daemon builds `ApiError`/failed-message text; unit test that each reason resolves to a non-empty localized string.
- [x] 6.5 Integration tests: existing fake-adapter suite still green (regression), plus an end-to-end test against the scripted fake OpenCode server (binding persisted after first turn, second turn reuses it, turn completes with text and a tool call).

## 7. Verification

- [x] 7.1 Manual E2E per `docs/FLOWS.md` with real OpenCode installed: startup spawns/binds the server, banner when unavailable, first send creates the binding, streamed markdown/tool cards render live, second send reuses the OpenCode session, daemon restart keeps context, killing the server mid-turn yields a failed message with human copy.
- [x] 7.2 Run the full workspace test suite and `scripts/check-crate-boundaries.py`; fix anything that regressed.
