# Tasks: opencode-provider-hardening

Review order follows proposal items 1–10. Each slice: investigate → implement → tests → manual check → commit (when user asks).

## 0. Prerequisites

- [x] 0.1 Refresh OpenAPI from live `GET /doc` (note version); update fixtures if event shapes changed — verified 2026-08-18 against live server v1.0.0 on :7433; key endpoints unchanged
- [x] 0.2 Resolve open questions in `design.md` (auto-title rule, item 8 scope, attach auth) with user
  - Auto-title: **yes**, default titles only
  - Item 8: **deferred** to separate change
  - Item 10: **deferred** (spawn-on-7433 unchanged)

---

## 1. Project-scoped CWD (proposal §1)

- [x] 1.1 Spike: document how OpenCode binds cwd/path (spawn vs session create vs PATCH) from live `/doc`
- [x] 1.2 Persist project folder path on Circulo project; expose to daemon on session generate
- [x] 1.3 Adapter: configure cwd/path per bound session before `prompt_async`
- [x] 1.4 Tests: fake server asserts prompt received with expected path context
- [ ] 1.5 Manual: session with project folder → agent tools see files in that folder

---

## 2. Abort generation (proposal §2)

- [x] 2.1 Extend `AgentAdapter` with `abort_turn`; implement `POST /session/:id/abort` in client
- [x] 2.2 Daemon: `POST /v1/sessions/{id}/abort` + interrupt in-flight generate on blocking pool
- [x] 2.3 App: composer stop control + locale keys; wire to daemon abort
- [x] 2.4 Tests: abort mid-scripted-turn ends turn with cancelled copy
- [ ] 2.5 Manual: long OpenCode turn → Stop → composer editable, no hung state

---

## 3. Mid-turn permissions (proposal §3)

- [x] 3.1 Map permission events from SSE; extend `AdapterEvent` + mapping tests
- [x] 3.2 Daemon: permission responder hook during supervised `run_turn`
- [x] 3.3 App: minimal Allow/Deny UI (modal or inline banner) + locale keys
- [x] 3.4 Client: `POST /session/:id/permissions/:permissionID`
- [ ] 3.5 Manual: supervised mode + tool that triggers ask → approve → turn continues

---

## 4. Opaque reasoning copy (proposal §4)

- [x] 4.1 Mapping: mark reasoning parts completed-without-text as opaque
- [x] 4.2 UI: `messages.reasoning_unavailable` in reasoning toggle
- [x] 4.3 Tests: fixture with empty reasoning + encrypted metadata → opaque state
- [ ] 4.4 Manual: model that encrypts reasoning shows honest copy

---

## 5. Delete OpenCode session sync (proposal §5)

- [x] 5.1 `AgentAdapter::delete_agent_session` + `DELETE /session/:id` client
- [x] 5.2 Daemon: call before local session delete; best-effort error log
- [x] 5.3 Tests: delete session removes binding and calls fake server delete
- [ ] 5.4 Manual: delete session in UI → OpenCode session count does not grow unbounded

---

## 6. Auto-title sync (proposal §6)

- [x] 6.1 Map `session.updated` title → `SessionTitleUpdated` event
- [x] 6.2 Daemon: persist title with overwrite rules (default titles only)
- [x] 6.3 App: sidebar reflects updated title via existing session list refresh
- [x] 6.4 Tests: fixture title update → persist + protocol event
- [ ] 6.5 Manual: first message → sidebar title matches OpenCode-generated title

---

## 7. OpenCode health/version (proposal §7)

- [x] 7.1 Client: `GET /global/health` parser
- [x] 7.2 Daemon `/v1/health`: include `opencode` sub-object
- [x] 7.3 Tests: fake server returns version string in health response
- [ ] 7.4 Manual: health JSON shows version when OpenCode running

---

## 8. Rich prompt parts (proposal §8) — out of scope

Deferred to follow-up change `opencode-attachments`. No tasks in this branch.

---

## 9. Todo refetch fallback (proposal §9)

- [x] 9.1 Client: `GET /session/:id/todo` + map to `TaskList`
- [x] 9.2 Invoke after SSE reconnect or turn recovery when todos may be stale
- [x] 9.3 Tests: todo endpoint reconciles after missed `todo.updated`
- [ ] 9.4 Manual: kill stream briefly mid-todo-turn → tasks still correct after recovery

---

## 10. Optional attach mode (proposal §10) — out of scope

Deferred. No tasks in this branch. Circulo keeps daemon-owned spawn on 7433.

---

## 11. Polish bundled

- [x] 11.1 Treat `server.heartbeat` as liveness reset in SSE read loop
- [x] 11.2 *(Optional)* Step indicators from `step-start` / `step-finish` — deferred (noisy; out of scope)

---

## 12. Verification

- [x] 12.1 Full workspace `cargo test` + `scripts/check-crate-boundaries.py`
- [ ] 12.2 Manual pass per `docs/FLOWS.md` for send, stop, supervised tool, delete session, project folder
- [x] 12.3 Update `EVENTS.md` mapping table for new events (permission, title, reasoning opaque)
