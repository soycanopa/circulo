# Circulo — Codex provider

- **Status:** Implemented (2026-11-26) against the documented `codex app-server`
  protocol; the real CLI is not installed on this machine yet, so the live
  probe is behind `--ignored`.
- **Crate:** `crates/providers/codex`, package `circulo-provider-codex`.
- **Source:** <https://developers.openai.com/codex/app-server> and the
  `app-server-protocol` schema in <https://github.com/openai/codex>.

## Transport: the app server over stdio

`codex app-server` is the interface Codex ships for rich clients (its own VS
Code extension uses it). It speaks JSON-RPC 2.0 over stdin/stdout, one message
per line, with the `"jsonrpc":"2.0"` header omitted on the wire. The default
`--listen stdio://` is what this driver uses: no port, no token, no WebSocket.

`codex exec --json` was the other candidate. It is a one-shot batch stream with
no client-side approvals and no interrupt, so it cannot answer a permission
card or stop a turn; the app server can.

## Handshake

1. `initialize` with `clientInfo` and `capabilities.experimentalApi: true`.
   The flag is required for `mcpServer/elicitation/request`, which is the only
   documented way Codex asks the client a question.
2. The `initialized` notification. The server rejects every other request on
   the connection until both have happened.

## A turn

1. **Thread.** With no stored key, `thread/start` with `cwd`,
   `approvalPolicy: "on-request"` and `sandbox: "workspace-write"`. With a
   stored key, `thread/resume` with `threadId`, the same `cwd` and
   `excludeTurns: true` (the transcript already lives in SQLite; loading the
   agent's turns again would only cost time). The answer carries
   `thread.id`, which is the continuity key.
2. **Turn.** `turn/start` with `threadId` and one text input, plus the stored
   `model` and `effort` overrides when there are any. The response carries
   `turn.id`; notifications stream until `turn/completed`.
3. **Finish.** `turn/completed` with `status: "completed"` ends the turn,
   `"interrupted"` becomes `DriverError::cancelled`, and `"failed"` carries the
   turn's `error.message`. A dead child ends the turn as a failure and is
   dropped, so the next turn resumes from the stored key.

Turns on one session queue behind each other, the same way OpenCode's do: the
session's slot is locked for the whole turn.

## Mapping

| app-server event | Circulo `Part` |
|---|---|
| `item/agentMessage/delta` | `Text { delta }` |
| `item/reasoning/summaryTextDelta` | `Reasoning { delta }` |
| `item/started` / `item/completed` with `commandExecution` | `Tool { id, name: "command", detail: command, state, output }` |
| `item/commandExecution/outputDelta` | accumulates into that tool's `output`, sent once with the finished row |
| `…` with `fileChange` | `Tool { id, name: "edit", detail: changed paths }` |
| `…` with `mcpToolCall` | `Tool { id, name: tool, detail: server }` |
| `…` with `webSearch` | `Tool { id, name: "web search", detail: query }` |
| `…` with `imageView` | `Tool { id, name: "image view", detail: path }` |
| `item/commandExecution/requestApproval` | a permission card; the answer is `{"decision": "accept" \| "acceptForSession" \| "decline" \| "cancel"}` |
| `item/fileChange/requestApproval` | a permission card with the same decisions |
| `mcpServer/elicitation/request` (form) | a question card; the answer is `{"action": "accept", "content": {…}}` |

`item/started` and `item/completed` both report the item, and both go through
the same row because the item id travels in `Part::Tool.id`. A `completed`
item is authoritative; `inProgress` is running. Unknown items, methods and
notifications are ignored: the app server adds them over time.

Text and reasoning arrive only as deltas; the accumulated `agentMessage` and
`reasoning` items are deliberately skipped, or every answer would be drawn
twice.

## Approvals

A command approval carries `command`, `cwd`, `reason` and, for managed
network access, `networkApprovalContext`. The card's title is the reason when
the agent gives one, otherwise "The agent wants to run a command"; the detail
is the command in full, falling back to the working directory. A managed
network prompt is titled "The agent is asking for network access" so it is not
mistaken for a shell command.

The four universal decisions are offered, in the protocol's order: `accept`
(Allow once), `acceptForSession` (Allow for session), `decline` (Reject) and
`cancel` (Stop the turn). `availableDecisions` is experimental and not used;
the four above are the documented baseline. A client that goes away answers
`cancel`, which also interrupts the turn — the turn was already being
abandoned.

File change approvals use the same four decisions and title
"The agent wants to edit files"; the detail is the reason, or the root the
agent asks to write under.

`item/permissions/requestApproval` (the `request_permissions` tool) is not
implemented: the request names a subset of network/filesystem permissions and
needs a different card. It is answered as unsupported, which fails that one
tool call rather than inventing a grant.

## Questions

`mcpServer/elicitation/request` is Codex's documented way for an MCP server to
ask a human. `mode: "form"` (and the `openai/form` variant) flattens
`requestedSchema` into fields exactly like ACP's form elicitation: an `enum`
becomes a choice, anything else a text field, and `required` decides whether
Submit waits. URL and device-verification modes are declined; this client has
no browser flow for them.

`tool/requestUserInput` is experimental and has a different shape (1–3 short
questions); it is answered as unsupported. A real Codex session only asks
through an MCP server unless the user runs that tool.

## Options

`model/list` gives each model's `id`, `displayName`, `supportedReasoningEfforts`
and `defaultReasoningEffort`. The driver renders two options:

- `model` — the catalog, current = the stored choice or the catalog default.
- `effort` — the selected model's supported efforts, current = the stored
  choice when that model supports it, otherwise the model's default.

Codex takes model and effort per turn, and a turn override becomes the default
for later turns. So `set_option` stores the choice and answers the refreshed
list; the next `turn/start` carries it. A choice made mid-turn lands on the
next turn, which is the contract the core already promises.

A model switch drops a stored effort the new model does not support, so the
picker never shows a value the agent would reject.

## Continuity

The key is `thread.id`, opaque to the core and stored in
`sessions.provider_session`. One `codex app-server` child per Circulo session
keeps the thread live across turns. After a daemon restart, `thread/resume`
reopens it; a key the agent no longer knows starts a fresh thread and the new
id heals the store. A dead child is dropped and the next turn resumes the same
way. Continuity degrades, it never fails a turn.

## Cancellation

`CancelTurn` reaches `Driver::cancel(session)`, which sends
`turn/interrupt` with the live `threadId` and `turnId`. The turn ends with
`status: "interrupted"`, which the driver reports as
`DriverError::cancelled`, so the core records `Aborted` with no error banner.

A cancel can arrive between `turn/start` and its response, before the turn id
exists. The handle remembers the request and the turn thread sends
`turn/interrupt` as soon as the id arrives, so the abort is never lost.

## Reset and shutdown

`reset` drops the session's child and slot: after a rewind the next prompt
starts a fresh thread over the restored files. The core also drops the stored
key, so nothing tries to resume it. `shutdown` drops every child; a child also
exits on stdin EOF, so no agent process outlives the daemon.

## Decisions

- **The app server, not `codex exec`.** Approvals, interrupt and thread resume
  are first class in the app server; `exec` has none of them.
- **`approvalPolicy: "on-request"`, `sandbox: "workspace-write"`.** The agent
  works in the project and asks before it leaves it, which is the product's
  existing permission model. `never` would silently drop the cards.
- **`experimentalApi: true`.** Only for the documented elicitation request;
  no experimental method is called by the driver.
- **No native steering.** Codex has `turn/steer`, but the core's steering is
  provider-neutral (abort the turn, send the text next). Using `turn/steer`
  would need a protocol change; it is left for a later slice.

## Verification

- Unit tests cover the mapping, the approval card and the question form.
- `tests/scripted.rs` runs a Python `codex app-server` stand-in: a turn
  streams (text, reasoning, a command's output), a permission card round trips,
  a question round trips, model and effort options apply, two turns share one
  child, a restart resumes the thread, a lost key starts fresh, cancel sends
  `turn/interrupt`, reset drops the thread, and shutdown releases the child.
- `cargo test -p circulo-provider-codex --test real -- --ignored` runs the
  same lifecycle against the installed `codex`; it needs a logged-in CLI.

## Limits, recorded on purpose

- **`codex` must be on `PATH`.** `CIRCULO_CODEX_BIN` points the driver at
  another binary (the scripted agent, a pinned build).
- **`model/list` may need a signed-in account.** Without one the picker is
  empty, but a turn still reports the spawn or auth failure honestly.
- **No `request_permissions` card, no URL elicitation, no
  `tool/requestUserInput`.** See above; each is answered as unsupported.
- **No tool diffs yet.** `commandExecution` output is captured; a `fileChange`
  item's `diff` is not. Usage (`thread/tokenUsage/updated`) is not rendered.
  The events exist; the wire has nowhere to put the diff or the gauge yet.
