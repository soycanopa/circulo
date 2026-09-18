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

## 2. Phases

| # | Work | Tests |
|---|---|---|
| 0 | Wire `CIRCULOGO_OPENCODE_BIN`; capture v2 fixtures (boot, sessions, real turn with tool, permission ask/reply, error) | fixtures header-record 2.0.8 |
| 1 | `wire.go` v2 types + `client.go` endpoints + Basic auth from captured password | httptest shape tests |
| 2 | `translate.go` v2 events → neutral protocol (neutral contract changes land with `protocol.ts` in the same commit) | table tests per event |
| 3 | `process.go`: read server password from output; `WaitHealthy` with auth; managed attach of the password to the client | process tests |
| 4 | UI: permission reply shape, model/variant mapping (`variants` array), any event renames in `protocol.ts` | reducer goldens |
| 5 | Docs: TRD pin → 2.0.x, this doc status, fixture headers | — |
| 6 | CI-lite + owner E2E manual pass | gate |

## 3. Decisions already taken by the owner
- Update everything to v2 (2026-09-18). v1 CLI stays installed for daily use until the
  app's pin moves.
