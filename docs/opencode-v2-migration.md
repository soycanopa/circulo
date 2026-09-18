# OpenCode v1 → v2 — Migration Plan

- **Status:** plan (2026-09-18); implementation in phases on `feature/opencode-v2`
- **Verified against:** live `opencode` **2.0.8** server (`~/.local/opencode-v2/`, npm `@opencode/cli`),
  `/openapi.json` (OpenAPI 3.1, 113 paths, 242 schemas) captured 2026-09-18
- **Source of truth rule:** unchanged (AGENTS.md) — re-verify against the running server spec
  and live captures before every assumption.

## 1. What changes (verified)

### Auth — breaking
- v2 `serve` prints `server password <random>` to output at boot and **requires HTTP Basic
  auth** on every request: user `opencode`, password = the printed one
  (`X-Api-Key`/Bearer → 401; verified).
- `/doc` is now an HTML page; the machine spec lives at **`/openapi.json`**.
- Adapter work: capture the password from the spawned process output, feed the existing
  `WithBasicAuth("opencode", pass)`; attach mode needs stored credentials (settings change).

### Binary strategy — side-by-side
- Keep the user's v1 CLI (`opencode-ai` npm, 1.18.31) untouched; v2 lives at
  `~/.local/opencode-v2/node_modules/@opencode/cli-darwin-arm64/bin/opencode`.
- `AdapterConfig.Binary` (already exists, unused) gets wired to env
  `CIRCULOGO_OPENCODE_BIN` in main.go; default stays `opencode` until v2 is the pin.

### Endpoint map (the 12 our client uses)
| v1 | v2 |
|---|---|
| `GET /global/health` | `GET /global/health` (still there; also `GET /api/info`) |
| `GET /path` | TBD → `GET /api/location` (worktree/location info) |
| `GET /session` | `GET /api/session` |
| `POST /session` | `POST /api/session` |
| `PATCH/DELETE /session/{id}` | `PATCH/DELETE /api/session/{sessionID}` |
| `GET /session/{id}/message` | `GET /api/session/{sessionID}/message` |
| `POST /session/{id}/prompt_async` | `POST /api/session/{sessionID}/prompt` |
| `POST /session/{id}/abort` | `POST /api/session/{sessionID}/interrupt` |
| `POST /session/{id}/permissions/{pid}` | `POST /api/session/{sessionID}/permission/{requestID}/reply` |
| `GET /agent` | `GET /api/agent` |
| `GET /config/providers` | `GET /api/provider` + `GET /api/model` |
| `GET /event` (SSE) | `GET /api/event` (SSE) |

### Event stream
- Frames: `{id, event: <name>, data: <json-string>}` — payload is a JSON string, event
  name in the SSE `event:` field. Idle server is silent (no v1-style heartbeats observed);
  our SSE keep-alive already covers proxies.
- Event names: not in the spec (opaque `V2EventEncoded`) — must be captured from a live
  turn (phase 0 fixture work, same discipline as the v1 fixtures).

### Schema deltas (spot-verified)
- `Session.Info`: adds `projectID`, `model: Model.Ref`, `cost`, `tokens`, `outcome`,
  `permissions: Permission.Ruleset`, `revert`, `location`.
- `Model.Info`: `capabilities` + `variants` is an **array of Model.Variant objects**
  (v1: string-keyed map); adds `enabled`, `canonical`, `compatibility`.
- `Permission.Request`: `{id, sessionID, action, resources, save, metadata, message}`
  — different shape from v1 `permission.asked`.
- Tool states now include `Streaming`; there are inbox/form/pty/vcs surfaces we ignore
  in v0 (neutral protocol stays minimal).
- `GET /api/experimental/migration/v1` exists — v2 can import v1 session data (test in
  phase 0; determines whether existing sessions survive).

## 1.1 Wire intel captured in phase 0 (live 2.0.8)

- SSE frames keep v1's layout (`data:` lines + `: heartbeat` comments) but the payload is
  an envelope `{id, type, data, created?, location?, durable?}` — the event name travels
  in `type`, and `durable` exposes the event-sourcing seq (aggregateID/seq/version).
- REST responses are wrapped: `{"data": …}` (v1 returned bare objects).
- `POST /api/session/{id}/prompt` body = `{text, files?, agents?, skills?, metadata?,
  delivery?, resume?}` — **no model/agent/variant**. Model is set on the session via
  `POST /api/session/{id}/model` with `{model: {id, providerID}}` (Model.Ref); agent via
  `/agent`. Wrong shapes 400 with `{"_tag":"InvalidRequestError", …}`.
- Turn event names (live turn): `user`, `session.execution.started/.succeeded`,
  `session.inbox.enqueued/.delivered`, `session.text.started/.delta/.ended`,
  `session.reasoning.started/.delta/.ended`,
  `session.tool.input.started/.ended`, `session.tool.called/.progress/.success`,
  `session.step.started/.streamed/.ended` (ended carries `finish`, `cost`,
  `tokens{input,output,reasoning,cache{read,write}}`),
  `session.usage.updated`, `session.renamed` (auto-title),
  plus config noise: `model/provider/command/skill/websearch.updated`,
  `mcp.status.changed`, `session.instructions.updated`, `shell.created/.exited`.
  `session.execution.failed` **not yet captured** (see phase 0 notes).
- Noise filter for translate: mcp/integration/instructions/shell/websearch/command/
  skill/provider/model `*.updated`, `mcp.status.changed`.
- Fixtures captured: `testdata/v2-session-lifecycle.sse`, `testdata/v2-turn-basic.sse`
  (real, 2.0.8). **Pending live capture:** permission ask/reply and execution failure —
  a scratch server auto-approves edits and a stuck session swallowed the failure; both
  must be captured during the owner E2E pass (v1 precedent: synthetic then replaced).

## 2. Phases

| # | Work | Status |
|---|---|---|
| 0 | `CIRCULOGO_OPENCODE_BIN` wired; real v2 fixtures captured (lifecycle + turn) | ✅ — permission/execution-failed captures pending owner E2E |
| 1 | `wire_v2.go` + `client_v2.go` (+ `client_v2_test.go` shape tests) | ✅ |
| 2+3 | `translate_v2.go` → neutral protocol; adapter swapped to the v2 client; `WaitReady` (boot password → Basic auth → /api/info); v1 code deleted | ✅ — neutral contract unchanged, `protocol.ts` needed no field changes |
| 4 | Reducer/store merge-patch for partial `session.updated` (v2 renamed patches) + tests | ✅ |
| 5 | Docs: TRD pin → 2.0.x, this doc status, fixture headers | pending |
| 6 | CI-lite ✅ + owner E2E manual pass (incl. permission + execution-failed captures) | gate |

## 3. Decisions already taken by the owner
- Update everything to v2 (2026-09-18). v1 CLI stays installed for daily use until the
  app's pin moves.
