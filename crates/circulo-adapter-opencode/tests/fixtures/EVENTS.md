# OpenCode server contract — observed on opencode 1.18.18 (2026-08-16)

Pinned from a live `opencode serve --hostname 127.0.0.1 --port 7480`. Schemas in
`openapi-excerpt.json` (trimmed from `GET /doc`); wire captures in `turn-text-tool.sse`
and `turn-todo.sse`. Regenerate by running a server, capturing `GET /event`, and
re-curating.

## Wire format

- SSE frames are `data: {json}\n\n`. There are **no named `event:` lines**; the
  discriminator is the JSON field `type`.
- Envelope: `{"id": "evt_...", "type": "...", "properties": {...}}`.
- `server.connected` (empty properties) is the first frame; `server.heartbeat`
  (empty properties) arrives periodically as keep-alive.

## Endpoints used by Circulo

- `POST /session` body `{}` → 200 `Session` (`id` matches `ses_*`). Optional
  query `directory=<path>` scopes the session working tree (OpenAPI 1.18.18).
  Defaults (agent `build`, model from user config) come from the server; we do
  not send model/agent in the create body.
- `POST /session/{sessionID}/prompt_async` body
  `{"parts": [{"type": "text", "text": "..."}]}` → **204 No Content**. Pass the
  same `directory` query param on each prompt so tools run in the Circulo project
  folder.
- `GET /event` → global SSE stream; every interesting event carries
  `properties.sessionID`, so we filter by our session id.
- `POST /session/{sessionID}/abort` → **200** `boolean`. Pass the same
  `directory` query param as prompts. Stops the in-flight turn without deleting
  the session.
- `POST /session/{sessionID}/permissions/{permissionID}` body
  `{"response": "once" | "reject"}` → **200** `boolean`. Pass the same
  `directory` query param. Unblocks a mid-turn permission prompt.
- `DELETE /session/{sessionID}` → **200** `boolean`. Pass the same
  `directory` query param when deleting a project-scoped session.
- `GET /session/{sessionID}/todo` → **200** array of `{content, status, priority}`
  Todo objects. Used to reconcile task cards when SSE missed `todo.updated`.
- `GET /doc` → OpenAPI 3.1 JSON; used as the liveness/identity probe (a 200 with
  an `openapi` field means a real OpenCode server).
- `GET /global/health` → `{ "healthy": bool, "version": string }`. Circulo
  surfaces this on daemon `/v1/health` as `opencode.available` / `opencode.version`.
- Auth: none for our own spawned server (we strip `OPENCODE_SERVER_PASSWORD` /
  `OPENCODE_SERVER_USERNAME` from the child env). A 401 anywhere maps to the
  `Unauthorized` adapter reason.

## Turn lifecycle (observed)

1. `message.updated` with `properties.info.role == "user"` (+ a
   `message.part.updated` for the user text part).
2. `session.status` `{status: {type: "busy"}}`.
3. Zero or more assistant **steps**. Each step is its own assistant message
   (`info.role == "assistant"`, `info.parentID` = user message id):
   - `message.part.updated` `part.type == "step-start"`
   - `part.type == "reasoning"` parts → `ReasoningDelta` (kept separate from reply text)
   - tool parts: `part.type == "tool"` with `part.tool` (e.g. `read`) and
     `part.state.status` flowing `pending → running → completed | error`
   - text parts: `part.type == "text"`, `part.text` is the **full accumulated
     snapshot** for that part id
   - `message.part.delta` `{sessionID, messageID, partID, field: "text", delta}`
     carries **true incremental deltas** for a text part; snapshots and deltas
     interleave and overlap
   - `part.type == "step-finish"`, then `message.updated` with
     `info.time.completed` set for that step
4. Turn terminus: `session.status` `{type: "idle"}` followed by `session.idle`
   `{sessionID}`. A failed turn surfaces `session.error` `{sessionID, error}`
   (error objects have a `name`, e.g. `ProviderAuthError`) and/or
   `message.updated` with `info.error` set; treat both as terminal-failure
   signals. `session.idle` can also follow an error, so error wins if seen.

## Mapped shapes

| OpenCode | Circulo `AdapterEvent` |
| --- | --- |
| `message.part.delta` (`field == "text"`, part **announced as `text`** by a prior `message.part.updated`) | `TextDelta` with `delta` |
| `message.part.delta` for any other/unannounced part id | skipped — reasoning parts also stream `field: "text"` deltas and are only distinguishable by their announcing snapshot (observed live: every part announces with a possibly-empty snapshot before its deltas) |
| `message.part.updated` reasoning snapshot / `message.part.delta` on announced reasoning parts | `ReasoningDelta` with suffix/delta text |
| `message.part.updated` reasoning with `time.end` and empty `text` | `ReasoningOpaque` (provider-hidden / encrypted) |
| `message.part.updated` text snapshot | `TextDelta` with the suffix beyond the last emitted offset for that `partID` |
| `message.part.updated` tool `state.status == "pending"/"running"` | `ToolCallStarted`/`ToolCallUpdated` (running) with input summary as context |
| `message.part.updated` tool `completed` | `ToolCallUpdated` succeeded; `state.output` string → `ToolOutput::Text` (diff output stays text; the renderer detects diffs) |
| `message.part.updated` tool `error` | `ToolCallUpdated` failed; `state.error` → `ToolOutput::Error` |
| `todo.updated` `{todos: [{content, status, priority}]}` | `TaskList` (statuses `pending`/`in_progress`/`completed`/`cancelled`) |
| `session.idle` (no prior error) | `Completed` |
| `session.error` or `info.error` | `Failed` |
| `permission.asked` / `permission.v2.asked` | blocks turn until `POST /session/:id/permissions/:permissionID` (`response`: `once` \| `reject`) |
| `permission.updated` (legacy) | same blocking path as `permission.asked` |
| `session.updated` (non-empty `info.title`) | `SessionTitleUpdated` |
| `server.connected` / `server.heartbeat` | consumed by adapter SSE reader (resets read idle timer; not forwarded) |
| everything else (`session.diff`, `step-*`, `reasoning`, plugin/lsp/mcp noise, unknown future types) | skipped, never fatal |

## Notable observations

- A turn produces **multiple assistant messages** (one per step). The
  user-visible reply is the concatenation of text parts across steps, in order;
  tool calls and todo updates interleave.
- The session title is auto-updated by the server during the turn
  (`session.updated` carries a generated `title`). Circulo persists it only when
  the local title is still the default (`New session`); manual renames are kept.
- Tool `state.output` is a plain string (text or JSON, sometimes a diff). No
  structured diff field exists on `ToolPart`.
- IDs: sessions `ses_*`, messages `msg_*`, parts `prt_*`, events `evt_*`.
