# circuloGo — Technical Requirements Document

- **Status:** Draft v0.1 (2026-09-02)
- **Related:** [PRD](prd.md) · [Flow](flow.md) · [Implementation Plan](implement.md)
- **Verified against:** OpenCode **1.18.25** — note: 1.18.25 uses `permission.asked`/`permission.replied` with a nested `data` payload (NOT the older `permission.updated` shape from the docs site), and `message.part.delta` exists on the v1 stream (live `opencode serve` + `GET /doc` OpenAPI 3.1,
  smoke-tested 2026-09-02), Wails **v3.0.0-beta.16** (`wails3 doctor` clean), Go 1.27, Node 24.

## 1. Stack

| Layer | Choice | Version pin | Why |
|---|---|---|---|
| Shell | Wails v3 | `v3.0.0-beta.16` (go.mod pin) | User decision. Beta is moving → pin + document gotchas (§10) |
| Backend | Go | ≥ 1.25 (1.27 installed) | Wails v3 requirement |
| Frontend | React + TypeScript + Vite | React 18, Vite 8 (template default) | `wails3 init -t react` baseline |
| Styling | Tailwind CSS | v4 (template default) | |
| Components | shadcn/ui (Radix) | latest | Sidebar/Dialog/Dropdown primitives |
| State | zustand | latest | Small event-reducer stores; no Redux ceremony |
| Markdown | react-markdown + remark-gfm | latest | Streaming-friendly (memoizable per message); shiki rejected for now: re-highlight cost on every delta |
| Tests | Go testing + httptest; vitest | — | Parsers/protocol first (project rule) |

**Rejected alternatives:** Tauri/Electron (user chose Wails); ACP stdio protocol (user
decision: HTTP+SSE); direct webview→`http://127.0.0.1` fetch (CORS — see §4).

## 2. Architecture

```
main.go                    Wails app: services, window, single instance
internal/
  agent/                   AgentAdapter interface + lifecycle types (no Wails imports)
  agent/protocol/          NEUTRAL WIRE CONTRACT: events + parts (JSON, camelCase).
                           Mirrored by hand in frontend/src/lib/agent/protocol.ts
  opencode/                ADAPTER: HTTP client, SSE consumer, process spawn, translation.
                           Imports: agent, agent/protocol, stdlib. Never Wails, never UI.
  orchestrator/            Registry: projects → adapter instance; fan-in of adapter events
                           to subscribers; persistence via store
  relay/                   http.Handler mounted at /agent (same-origin API for the UI):
                           GET /agent/sse + command endpoints. No business logic — it
                           translates HTTP ↔ orchestrator calls.
  store/                   settings.json read/write (projects, connection modes)
frontend/src/
  lib/agent/protocol.ts    TS mirror of internal/agent/protocol (the only shared contract)
  lib/agent/sse.ts         EventSource wrapper: connect, auto-reconnect, dispatch
  lib/agent/reducer.ts     events → SessionState (pure functions, vitest-covered)
  stores/                  zustand: projects, sessions, chat (thin shells over reducer)
  components/sidebar/      ProjectSwitcher, SessionList (grouped by date), context menus
  components/chat/         Transcript, MessageGroup, PartRenderer (+ per-part components),
                           Composer, PermissionCard, StatusStrip
  components/ui/           shadcn primitives
```

**Dependency rule (enforced by review + import-linter mindset):**
`frontend → protocol ← orchestrator ← opencode`. The UI knows *parts*, never OpenCode.
Adding adapter #2 touches only `internal/<newadapter>` + one registration line.

## 3. OpenCode adapter (v1 API, verified 1.18.25)

### 3.1 Process management (mode: managed)

- Spawn: `opencode serve --hostname 127.0.0.1 --port <free>` with `Dir` = project path;
  free port from `net.Listen(":0")` closed immediately before spawn (TOCTOU acceptable
  locally; retry once on bind failure).
- Readiness: poll `GET /global/health` (`{"healthy":true,"version":…}`) every 250 ms up
  to 10 s. The server prints `opencode server listening on http://…` on stdout.
- Stdout/stderr: captured to a ring buffer (last 4 KB) for error reporting (FR-19).
- Shutdown: `cmd.Process.Kill()` after graceful 2 s wait; orchestrator stops all adapters
  on `ServiceShutdown()` (note: signature is `ServiceShutdown() error` — no ctx; Wails
  silently skips a `ServiceShutdown(ctx)` method).
- Attach mode: user-provided base URL, no spawn, no kill. Health-check on connect only.
- Optional auth: if `OPENCODE_SERVER_PASSWORD` is set in the adapter config, send
  `Authorization: Basic base64("opencode:"+password)` on every request (username
  overridable via `OPENCODE_SERVER_USERNAME`). Default: no auth (loopback only).

### 3.2 Verified API surface used

| Call | Endpoint (OpenCode v1) |
|---|---|
| Health | `GET /global/health` |
| Event stream | `GET /event` (SSE; first event `server.connected`; heartbeat events ignored) |
| Create session | `POST /session` `{title?}` → `Session` |
| List sessions | `GET /session` → `Session[]` |
| Rename | `PATCH /session/{id}` `{title}` |
| Delete | `DELETE /session/{id}` |
| History | `GET /session/{id}/message?limit=` → `{info: Message, parts: Part[]}[]` |
| Send prompt | `POST /session/{id}/prompt_async` `{model:{providerID,modelID}, agent, parts:[{type:"text",text}]}` → **204** |
| Abort | `POST /session/{id}/abort` |
| Answer permission | `POST /session/{id}/permissions/{permissionID}` `{response:"once"\|"always"\|"reject"}` |
| Agents | `GET /agent` |
| Providers/models | `GET /config/providers` |

Source of truth for schemas: the server itself (`GET /doc`, OpenAPI 3.1) — never guess;
re-verify when the pinned opencode version changes.

### 3.3 SSE facts (captured live, 1.18.25)

- Frames are `data: {…}\n\n` with **no** `event:` line. Envelope:
  `{"id":"evt_…","type":"message.part.updated","properties":{…}}`.
- Turn lifecycle observed: user text part echo → `session.status busy` → assistant
  `message.updated` → `step-start` part → `text` part (empty) → `message.part.delta`
  (`{sessionID, messageID, partID, field:"text", delta}`) → … → final full text part →
  `step-finish` part (tokens+cost per step) → final `message.updated` →
  `session.status idle` → `session.idle`.
- `message.part.updated` carries the **full part** → missing events self-heal; `delta`
  is an optimization only.
- Retry visibility: `session.status` `{type:"retry", attempt, message, next}` (verified
  with a real 429 from a provider plan restriction) and `session.error`
  `{sessionID, error:{name:"APIError", data:{message, statusCode, isRetryable, …}}}`.
- Ignore-list (log at debug, never crash): `server.heartbeat`, `plugin.added`,
  `catalog.updated`, `reference.updated`, `integration.updated`, `lsp.*`, `file.*`,
  `todo.updated`, `pty.*`, `installation.*`, `vcs.branch.updated`, `tui.*`, `command.executed`.

## 4. Transport: backend → webview

The webview origin is `wails://localhost` (macOS/Linux). `fetch`/`EventSource` from the
page to `http://127.0.0.1:<port>` is cross-origin and blocked unless the target sends
permissive CORS. OpenCode has `--cors`, but routing agent traffic through the webview
would couple the UI to OpenCode's network location and leak adapter details.

**Decision:** one Wails service (`relay`) implements `http.Handler` and is mounted at
route `/agent` (`application.ServiceOptions{Route:"/agent"}`). The UI talks same-origin:

```
GET  /agent/sse                     SSE stream of neutral events (all projects; envelope
                                    carries projectID — mirrors /global/event semantics)
GET  /agent/projects                configured projects + adapter status
POST /agent/projects                {path, mode: managed|attach, url?} → project
DELETE /agent/projects/:id
GET  /agent/projects/:id/sessions
POST /agent/projects/:id/sessions   {title?} → session
PATCH/DELETE /agent/projects/:id/sessions/:sid
GET  /agent/projects/:id/sessions/:sid/messages     (hydration)
POST /agent/projects/:id/sessions/:sid/prompt       {text, agent?, model?}
POST /agent/projects/:id/sessions/:sid/abort
POST /agent/projects/:id/sessions/:sid/permissions/:pid   {response}
GET  /agent/projects/:id/meta       {agents, providers, models}
```

No CORS, standard `EventSource`, fully testable with `httptest`. Fallback (decide in
Phase 3): if route-mounted handlers misbehave under `wails3 dev`, switch the stream to
`app.Event.Emit` per event — commands stay on bindings. This is a contained swap behind
`relay`.

## 5. Neutral protocol (contract `internal/agent/protocol` ⇄ `protocol.ts`)

Envelope over `/agent/sse` (`data:` JSON, camelCase):

```jsonc
{ "type": "part.updated", "payload": { … } }   // every event
```

Event types:

| `type` | `payload` |
|---|---|
| `adapter.status` | `{projectID, state: "starting"\|"running"\|"stopped"\|"error", detail?}` |
| `session.updated` | `{projectID, session: Session}` (also fired for created/deleted; `Session` carries `title, status, time{created,updated}`) |
| `session.status` | `{projectID, sessionID, status: "idle"\|"busy"\|"retry", retry?{attempt, message, nextAt}}` |
| `message.updated` | `{projectID, sessionID, message: MessageInfo}` (role, tokens, cost, finish, error) |
| `part.updated` | `{projectID, sessionID, messageID, part: Part}` (full replacement) |
| `part.delta` | `{projectID, sessionID, messageID, partID, field, delta}` (best-effort fast path) |
| `permission.request` | `{projectID, sessionID, permission: {id, kind, title, pattern?, metadata, callID?}}` |
| `permission.resolved` | `{projectID, sessionID, permissionID, response}` |
| `session.error` | `{projectID, sessionID?, error: {name, message, recoverable?}}` |
| `session.removed` | `{projectID, sessionID}` |

`Part` union (inspired by OpenCode parts — see smoke test in §3.3):

```jsonc
{ "id": "…", "type": "text",      "text": "…", "time": {"start":0,"end":0} }
{ "id": "…", "type": "reasoning", "text": "…", "time": {"start":0,"end":0} }
{ "id": "…", "type": "tool", "callID": "…", "tool": "bash",
  "state": { "status": "pending"\|"running"\|"completed"\|"error",
             "input": {}, "title"?: "…", "output"?: "…", "error"?: "…",
             "metadata"?: {}, "time"?: {"start":0,"end":0} } }
{ "id": "…", "type": "step-start" }
{ "id": "…", "type": "step-finish", "reason": "stop", "cost": 0,
  "tokens": {"input":0,"output":0,"reasoning":0,"cache":{"read":0,"write":0}} }
{ "id": "…", "type": "patch", "hash": "…", "files": ["a.go","b.ts"] }
{ "id": "…", "type": "agent", "name": "explore" }
{ "id": "…", "type": "subtask", "prompt": "…", "description": "…", "agent": "…" }
{ "id": "…", "type": "file", "mime": "text/plain", "path": "src/x.go" }
```

Unknown `type` values are skipped by both Go and TS decoders (NFR-3).

### 5.1 Translation table (OpenCode → neutral)

| OpenCode event/payload | Neutral |
|---|---|
| `server.connected` | `adapter.status{running}` |
| `session.created` / `session.updated` / `session.deleted` | `session.updated` / `session.removed` |
| `session.status` `{idle,busy}` | `session.status` |
| `session.status` `{type:"retry",attempt,message,next}` | `session.status{retry:{attempt,message,nextAt}}` |
| `message.updated` `{info}` | `message.updated` (map AssistantMessage fields) |
| `message.part.updated` `{part}` | `part.updated` (map part, table below) |
| `message.part.delta` `{…,field,delta}` | `part.delta` |
| `permission.asked` (Permission object, nested `data`) | `permission.request` |
| `permission.replied` | `permission.resolved` |
| `session.error` `{error:{name,data:{message,…}}}` | `session.error` |
| `session.idle` | folds into `session.status{idle}` (UI uses it as turn-end) |

Part mapping: `text→text`, `reasoning→reasoning`, `tool→tool` (state.status passthrough,
`state.metadata` passthrough as opaque JSON), `step-start→step-start`,
`step-finish→step-finish`, `patch→patch`, `agent→agent`, `subtask→subtask`,
`file→file` (path from `source.path` when present), `snapshot`/`retry`/`compaction` →
dropped in v0 (retry surfaces via `session.status`; compaction UI is v1).

## 6. Frontend data model

```ts
interface SessionState {
  session: Session;
  status: "idle" | "busy" | "retry";
  retry?: { attempt: number; message: string; nextAt: number };
  messages: MessageRecord[];          // ordered by arrival; hydrated history first
  permissions: PermissionRequest[];   // unresolved only
}
interface MessageRecord { info: MessageInfo; parts: Map<string, Part>; partOrder: string[]; }
```

- Identity: `(messageID, partID)`. `part.updated` upserts by id and appends to
  `partOrder` on first sight → one reducer serves live streaming and hydration (FR-14).
- `part.delta` appends to the existing part's text/reasoning field; a following
  `part.updated` is authoritative and replaces it (idempotent under reordering).
- Render batching: the store pushes into a micro-queue flushed on
  `requestAnimationFrame` (~15 Hz cap for transcript re-renders); composer and status
  render immediately.
- Markdown: memoized per message; only the last text part of the streaming message
  re-parses.

## 7. Persistence (`internal/store`)

`settings.json` under OS config dir (`~/Library/Application Support/circuloGo/` on macOS):

```jsonc
{ "projects": [ { "id": "…", "path": "/…", "mode": "managed"|"attach", "url"?: "…" } ],
  "ui": { "lastProjectId": "…" } }
```

No chat DB in v0 — history lives in the OpenCode server's own storage and is hydrated
via `GET /session/{id}/message` (FR-8). Consequence: deleting a project's OpenCode data
deletes its history; acceptable and documented.

## 8. Testing strategy

- **Go — protocol first (Phase 2):** fixtures in `internal/opencode/testdata/` are real
  captured SSE lines (server.connected, session.*, message.part.updated for every part
  type, message.part.delta, permission.*, retry status, session.error). Parser tests +
  translation table tests + `httptest` client tests + spawn-lifecycle test (skipped if
  `opencode` not on PATH).
- **TS — reducer first (Phase 2/5):** vitest drives `reducer.ts` with synthetic streams
  (golden transcript): delta-then-full, out-of-order parts, unknown event/part types,
  permission lifecycle, retry status, hydration-then-live overlap.
- **Manual E2E (Phase 3+):** `docs/implement.md` §E2E checklist against a real
  `opencode serve` in a scratch project.

## 9. Security

- Adapter binds to 127.0.0.1 only; relay is same-origin via the Wails asset server (no
  extra listener; nothing to discover from the LAN).
- Optional Basic auth passthrough (`OPENCODE_SERVER_PASSWORD`).
- No telemetry, no outbound calls other than the agent server.
- Future remote access (Tailcat, design in [remote.md](remote.md)) will expose exactly one
  surface — a static-UI + relay listener — behind a token-gated tunnel; adapters and
  `opencode serve` processes are never reachable off-machine.

## 10. Known risks & gotchas (verified where noted)

1. **Wails v3 is beta** — pin `v3.0.0-beta.16`; `ServiceShutdown() error` must not take
   ctx (silently never called); `frontend/bindings/` is generated — never hand-edit;
   `Events.Off(name)` removes *all* listeners (keep unsubscribe fns).
2. **Route-mounted service in dev** — verify `GET /agent/sse` interception under
   `wails3 dev` early (Phase 3); fallback = `app.Event.Emit` (§4).
3. **OpenCode `/event` regressions** — GitHub issues #26866/#26697 reported versions
   streaming only `server.connected`. Mitigation: smoke test at Phase 2 start
   (already done for 1.18.25 ✓), record fixture set, re-run on version bump.
4. **Full-part traffic** — v1 replaces whole parts; long tool outputs re-serialize each
   update. Accepted: payloads are tens of KB worst-case on loopback; revisit if v2
   granular events are adopted.
5. **Free-port race** — retry-once logic; failure surfaces as project error state (FR-19).
