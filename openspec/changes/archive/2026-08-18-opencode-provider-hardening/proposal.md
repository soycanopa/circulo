# opencode-provider-hardening

## Why

The first OpenCode adapter (`adapter-opencode`) made real chat possible, but day-to-day use exposed ten gaps between Circulo and the [OpenCode server API](https://opencode.ai/docs/server). The agent often runs in the wrong folder, turns cannot be stopped, supervised permissions stall mid-turn, reasoning blocks look broken when encrypted, and orphaned OpenCode sessions accumulate. Closing these gaps is the highest-leverage work left before MVP hardening: they directly affect whether a non-technical user can trust Circulo with real project work.

## What Changes

Ten improvements, grouped by user impact. Each is specified, designed, and task-sliced in this change; implementation may land in ordered commits but stays one branch.

| # | Improvement | User-visible outcome |
| --- | --- | --- |
| 1 | **Project-scoped CWD** | OpenCode runs with the Circulo project folder as its working directory when the session has a project; unassigned sessions keep a safe default. |
| 2 | **Abort generation** | User can stop a long turn; adapter calls `POST /session/:id/abort`. |
| 3 | **Mid-turn permission responses** | Supervised mode can answer OpenCode permission prompts instead of hanging. |
| 4 | **Opaque reasoning copy** | When reasoning is provider-encrypted, the UI shows honest copy instead of an empty toggle. |
| 5 | **Session delete sync** | Deleting a Circulo session also deletes the bound OpenCode session when a binding exists. |
| 6 | **Auto-title sync** | OpenCode-generated session titles flow into Circulo sidebar cards when the server emits them. |
| 7 | **OpenCode health/version** | Daemon health (and Settings when built) surfaces OpenCode `{ healthy, version }` via `GET /global/health`. |
| 8 | **Rich prompt parts** | *(Deferred — separate change `opencode-attachments`.)* |
| 9 | **Todo refetch fallback** | After stream recovery, task lists can be reconciled from `GET /session/:id/todo` if SSE missed updates. |
| 10 | **Optional attach mode** | *(Deferred — future power-user opt-in; default spawn unchanged.)* |

Polish bundled where cheap: treat `server.heartbeat` as liveness (extend read-idle budget), document dedicated port 7433 vs OpenCode default 4096.

## Capabilities

### New Capabilities

(none — all work extends existing surfaces)

### Modified Capabilities

- `opencode-adapter`: project CWD, abort, permission responses, OpenCode session delete on Circulo delete, health probe, todo refetch helper, heartbeat-aware streaming, title events from `session.updated`.
- `composer-stream`: cancel/stop control while generating (PRD-CHT-09 moves toward P0 for this change).
- `sessions-sidebar`: display synced auto-titles; session delete triggers adapter cleanup.
- `rich-message-render`: opaque reasoning state and locale copy.
- `local-daemon-api`: expose OpenCode version/health in `/v1/health` payload when adapter is OpenCode.
- `local-persistence`: optional metadata if title sync needs explicit provenance (only if design requires it).

## Impact

- **Crates:** `circulo-adapter-opencode` (major), `circulo-adapter` (trait extensions: abort, delete agent session, permission channel), `circulo-daemon` (HTTP handlers, delete hook, health), `circulo-app` (composer stop, reasoning copy, sidebar title), `circulo-persist` (minimal), `circulo-i18n`, `circulo-protocol` (if health/permission events cross the app boundary).
- **External API:** OpenCode server endpoints documented at opencode.ai — verify against live `/doc` before each slice.
- **Product decisions:** Item 6 closed (default titles only). Items 8 and 10 deferred to separate/future changes.

## Non-goals

- Replacing the fake adapter or adding providers.
- Full `QuestionCard` / interactive question UI (permission prompts are narrower).
- Image drag-and-drop in the composer (item 8 — separate change).
- Attach to user-started `opencode serve` (item 10 — deferred; Circulo keeps spawn-on-7433).
- Windows/Linux OpenCode lifecycle.
- App↔daemon TLS (unchanged).

## Open questions

~~1. **Auto-title (item 6):** Accept OpenCode's generated title as Circulo's session title on first emit, overwriting "New session"? *(Recommended: yes, user can still rename.)*~~

**Closed (2026-08-17):** Yes. OpenCode may replace only default titles (e.g. "New session"). Manual renames are never overwritten.

~~2. **Rich prompts (item 8):** Include in this change or a follow-up `opencode-attachments` change?~~

**Closed (2026-08-17):** Deferred to a separate change; out of scope for this branch.

~~3. **Attach mode (item 10):** Ship as `CIRCULO_OPENCODE_ATTACH=host:port` with no spawn, or also support password-protected servers?~~

**Closed (2026-08-17):** Deferred. Default spawn-on-7433 unchanged; attach mode out of this branch.
