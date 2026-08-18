# Design: opencode-provider-hardening

## Context

See `proposal.md`. The adapter crate already manages spawn, SSE mapping, model/agent/variant on `prompt_async`, and permission rulesets via `PATCH /session/:id`. The app talks only to the daemon; all OpenCode HTTP stays in `circulo-adapter-opencode`. Fixtures live in `tests/fixtures/EVENTS.md`; verify against live `/doc` when implementing each slice.

**Current gaps (verified against [OpenCode server docs](https://opencode.ai/docs/server)):**

| Item | Today | OpenCode API |
| --- | --- | --- |
| CWD | `$HOME` / `CIRCULO_OPENCODE_CWD` global | Session `path` in events; server cwd-sensitive |
| Abort | Not implemented | `POST /session/:id/abort` |
| Permissions | Pre-turn PATCH only | `POST /session/:id/permissions/:permissionID` mid-turn |
| Delete | Local CASCADE only | `DELETE /session/:id` |
| Title | Ignored `session.updated` | Server auto-titles sessions |
| Health | `/doc` probe only | `GET /global/health` → `{ healthy, version }` |
| Attach | Spawn-only on 7433 | Connect to existing `opencode serve` |
| Todo fallback | SSE `todo.updated` only | `GET /session/:id/todo` |
| Heartbeat | Ignored; 90s read timeout | `server.heartbeat` on `/event` |

## Goals / Non-Goals

**Goals:**

- Close proposal items 1–7 and 9 with test coverage at the adapter layer and manual checks for UI slices.
- Extend `AgentAdapter` minimally (abort, delete remote session, optional permission callback) without async trait refactor.
- Keep fake adapter usable for CI; extend fake to simulate abort/permission/title where needed.

**Non-Goals:**

- Item 8 (rich prompt parts / images) — **deferred** to follow-up change.
- Item 10 (attach mode) — **deferred**; Circulo keeps daemon-owned spawn on 7433.
- QuestionCard for generic OpenCode questions.
- Changing spawn ownership: Circulo always owns spawn in this change.

## Decisions

### D1. Project CWD via OpenCode `directory` query param (item 1)

**Choice:** OpenCode 1.18.18 scopes working trees per request via the optional `directory` query parameter on `POST /session`, `PATCH /session/:id`, and `POST /session/:id/prompt_async` (not a spawn-time cwd). Circulo persists the picked folder on `Project.folder_path`, resolves it in the daemon for each turn, and passes it as `directory` on session create, permission sync, and prompt. Unassigned sessions use `$HOME` (or `CIRCULO_OPENCODE_CWD` when set).

**Spike (2026-08-17):** Live `/doc` excerpt in `openapi-excerpt.json`; PATCH body does not carry path — query param is the supported mechanism. Per-project server instances not required for v1.

### D2. Abort via trait method + composer stop (item 2)

**Choice:** Add `AgentAdapter::abort_turn(agent_session_id) -> Result<(), AdapterError>` with default no-op for fake. Daemon exposes `POST /v1/sessions/{id}/abort` (or piggybacks on existing generate cancellation token). App composer swaps send button for stop while generating (like Waku).

Turn loop in `generate()` must be interruptible: abort sets a flag checked between SSE reads; on abort, call OpenCode abort and emit `Failed` with reason `Cancelled` (new `ErrorReason` or reuse `ProviderFailed` with distinct locale key).

### D3. Permission events cross daemon → app (item 3)

**Choice:** Map permission SSE events to new `AdapterEvent::PermissionRequested { id, summary, … }`. Daemon forwards on protocol SSE as a new event type **or** blocks in adapter until daemon responds (sync callback).

- **Recommended:** Sync blocking in adapter with a channel: daemon `generate` passes `permission_responder: &dyn Fn(PermissionRequest) -> PermissionResponse` into adapter for supervised mode only. App shows a lightweight modal/toast with Allow/Deny (not full QuestionCard). Timeout → deny with human copy.

**Alternative:** Fully async permission UI — rejected for scope; requires protocol + app state machine.

### D4. Opaque reasoning (item 4)

**Choice:** Extend `MessagePart::Reasoning` with `visible: bool` or detect empty content after `time.end` in mapping. UI uses `messages.reasoning_unavailable` locale key. No decryption attempts.

### D5. Delete OpenCode session on Circulo delete (item 5)

**Choice:** In daemon `DELETE /v1/sessions/{id}`, after loading binding, call adapter `delete_agent_session(id)` before SQLite delete. Best-effort: log and continue local delete on remote failure.

### D6. Auto-title (item 6)

**Choice:** Map `session.updated` where `properties` includes title → `AdapterEvent::SessionTitleUpdated { title }`. Daemon updates session row only when title is still the default/new-session pattern. Manual renames MUST NOT be overwritten (user-confirmed 2026-08-17).

**Closed:** Overwrite only from default titles; track via default-title match or explicit `title_source` if needed.

### D7. Health enrichment (item 7)

**Choice:** `OpenCodeClient::global_health()` → `{ healthy, version }`. Daemon `/v1/health` adds `opencode: { available, version }` JSON field. App Settings (future slice) reads it; sidebar daemon-down copy unchanged.

### D8. Rich prompts (item 8) — deferred

**Choice:** Out of scope for this change. Follow-up change `opencode-attachments` when approved.

### D9. Todo refetch (item 9)

**Choice:** After stream reconnect in adapter turn loop (or daemon stream recovery), if last event was tool/todo-heavy, call `GET /session/:id/todo` once and emit `TaskList`. App stream recovery already refetches messages — ensure daemon persistence includes todos from refetch path.

### D10. Attach mode (item 10) — deferred

**Choice:** Not in this branch. Circulo continues spawn-or-reuse on 7433 only. A future change may add `CIRCULO_OPENCODE_ATTACH=host:port` for power users who already run `opencode serve` in a terminal.

### D11. Heartbeat (polish)

**Choice:** On `server.heartbeat` SSE event, reset idle read deadline in turn loop (do not emit to UI).

## Risks / Trade-offs

- [CWD API unclear] → Spike task 1.0: read live OpenAPI before coding; may need per-project server instances.
- [Permission UI blocks turn] → Timeout with deny; show "Permission timed out" copy.
- [Auto-title overwrites user rename] → Only overwrite default titles; add `title_locked` flag if needed.
- [Attach mode security] → N/A while deferred.
- [Trait growth] → Keep new methods on adapter trait with default impls; fake stays simple.

## Migration Plan

1. Ship adapter/daemon changes behind existing `CIRCULO_ADAPTER=opencode`; fake tests unchanged.
2. DB migration only if `title_source` column needed (nullable, default auto).
3. Rollback: revert branch; spawn behavior unchanged.

## Open Questions

~~1. Auto-title overwrite rule (D6) — confirm with user.~~ **Closed:** default titles only; manual renames never overwritten.

~~2. Item 8 in or out of this change.~~ **Closed:** separate change `opencode-attachments`.

~~3. Attach mode (D10): include in this branch, defer, or drop?~~ **Closed:** deferred; spawn-on-7433 only in this branch.
