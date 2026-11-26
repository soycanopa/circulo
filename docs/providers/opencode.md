# Circulo — OpenCode provider

- **Status:** Implemented (2026-11-21), verified against OpenCode 2.3.2.
- **Crate:** `crates/providers/opencode`, package `circulo-provider-opencode`.

## Transport: Agent Client Protocol over stdio

The driver spawns `opencode acp` in the project directory and speaks JSON-RPC
2.0 over stdin and stdout, one message per line. That is the surface OpenCode
ships for editors and apps.

The v2 HTTP API (`opencode serve`, `/api/session`, `/api/event`) was the first
route, and it is not usable from a third-party client without more work:
the server answers `401 Authentication required`, the Basic token is not in the
announcement line, and the documented auth path is pairing
(`POST /api/pair`, `GET /auth/connect/{code}`). ACP has no port, no token, and a
documented message set, so it is the honest integration.

## What a turn does

1. `initialize` with `protocolVersion: 1`. The agent must answer with the same
   version; anything else fails the turn instead of guessing.
2. The session, only when there is no live one for the Circulo session yet:
   `session/new` with `cwd` and an empty `mcpServers` list — the answer carries
   a `sessionId` and `configOptions` (models, effort, modes) behind the
   pickers — or, when the core holds a stored key, `session/load` with that id,
   which resumes the agent's own history (see "Session continuity").
   The stored choices are re-applied with `session/set_config_option` in both
   cases, because a set lasts only for the session that received it.
3. `session/prompt` with one text part. Updates stream as
   `session/update` notifications until the response arrives.
4. The process stays alive: one `opencode acp` child per Circulo session,
   released when the daemon winds down (the driver's `shutdown`).

## Session continuity

The driver keeps one live agent per Circulo session — the child process and
its ACP session — so the agent remembers the conversation across turns. The
ACP session id is the key the driver hands back each turn; the core stores it
in `sessions.provider_session` and hands it back on the next turn, wherever
that happens:

- **Same daemon:** the live agent is reused as is; no session call at all.
- **Restarted daemon:** the stored key drives `session/load`, and the agent's
  own history continues. The replay arrives as `session/update` notifications
  before the response and is skipped — the transcript already lives in the
  database, and drawing it again would double it on screen. Probed on 2.3.2:
  the load response carries the session's `configOptions` but **no
  `sessionId`**; the loaded session is the one that was asked for.
- **Key the agent no longer knows:** the driver starts a fresh conversation
  rather than failing the turn, and the fresh key comes back so the store
  heals. Continuity degrades instead of breaking.
- **Dead child** (a crash): the slot is dropped and the next turn starts
  over, resuming through the stored key the same way.

Turns on one session queue behind each other — the agent answers one prompt
at a time — while other sessions keep moving. A model or effort switch made
while a turn is running is queued and lands at the turn's end: the picker
sees its value at once, the session reads it from the next turn.

## Cancellation

`CancelTurn` reaches the driver as `Driver::cancel(session)`. The turn thread
owns the session's slot for the whole turn, so the driver keeps a second handle
— the child's stdin behind a mutex and the ACP session id — that another thread
can use to send the ACP notification:

```json
{"jsonrpc":"2.0","method":"session/cancel","params":{"sessionId":"sess_…"}}
```

The core cancels the turn's open cards first, so a permission the agent is
waiting on is answered `cancelled`, as ACP requires. The agent then responds to
`session/prompt` with `stopReason: "cancelled"`; the driver turns that into
`DriverError::cancelled`, and the core reports `FinishReason::Aborted` without
an error banner. Parts already sent stay in the transcript; the child stays
alive for the next turn. An agent that returns an error instead of `cancelled`
(the ACP docs warn about libraries doing this) still ends the turn, but as a
failure.

## Mapping

| ACP `sessionUpdate` | Circulo `Part` |
|---|---|
| `agent_message_chunk` | `Text { delta }` |
| `agent_thought_chunk` | `Reasoning { delta }` |
| `tool_call`, `tool_call_update` | `Tool { id, name, detail, state }`, one row per `toolCallId` |
| `session/request_permission` | a permission card; the answer goes back as the chosen `optionId` |
| `elicitation/create` (form) | a question card; `accept` returns the values keyed by schema property |

`detail` prefers the tool's first location path, then `rawInput.path`. A tool is
`Running` while pending or in progress, `Done` when completed or failed; a
failure appends `· failed` to the detail. Unknown update kinds are ignored, so
ACP can add them without breaking the driver.

A permission request blocks the turn until the user answers. The card's title is
the ACP `toolCall.title`, its detail is `rawInput.command`, then the first
location path, then `rawInput.path`. When the agent offers no option at all the
driver answers `cancelled` rather than showing a card with no buttons. OpenCode
2.3.2 asks for `write` and `bash` calls; a project-level `permission` config does
not change that in ACP mode.

The ACP `toolCallId` travels in `Part::Tool.id`. Without it the transcript can
only match an update against the row that is still open at the end of the turn
parts, and a real turn interleaves its calls: A opens, B opens, then A finishes.
The completion of A then had no row to land on, which left A spinning and opened
a duplicate row for it.

## Limits, recorded on purpose

- **One process per Circulo session.** It lives with the daemon: a quit
  releases every child (driver `shutdown`, plus the agent closing on stdin
  EOF either way), and a crash of a child costs one failed turn before the
  next one starts over.
- **Permissions ask the user.** `session/request_permission` becomes a card and
  the turn blocks on the answer; a client that leaves reads as cancelled.
  Reads still work without asking: OpenCode's defaults allow them.
- **A set lasts for the agent session that received it:** a fresh
  `session/new` resets `effort` to `default` (probed on 2.3.2). The driver
  stores the user's choices and re-applies them whenever it attaches — to a
  new session, or to a loaded one. A stale choice the agent refuses is
  dropped.
- **No timeouts.** A wedged agent blocks its connection thread; the daemon has
  one thread per connection, so it does not block another client.
- **`opencode` must be on `PATH`.** The daemon reports the spawn failure as a
  turn error.

## How to run it

```sh
CIRCULO_PROVIDER=opencode CIRCULO_PROJECT="$PWD" cargo run -p circulo
```

The daemon reads both variables; without `CIRCULO_PROVIDER` it uses the demo
provider, so the window works without an agent installed.

## Verification

- Unit tests cover the mapping with fixtures captured from the real binary.
- `tests/continuity.rs` runs the lifecycle against a scripted ACP agent that
  counts prompts per process and keeps sessions in a directory across
  processes: two turns share one child, the key rides back, a live choice
  holds, a mid-turn choice lands next turn, shutdown releases the child
  (signal probing, not file presence — a SIGKILL runs no cleanup), a dead
  child is dropped, a restart resumes, a lost session starts fresh.
- `cargo test -p circulo-provider-opencode --lib -- --ignored` reattaches to
  a real session across two processes without spending a model call.
- `cargo test -p circulo-provider-opencode --test real -- --ignored` runs the
  model-call tests: a turn, remembering across turns, and resuming after a
  driver restart (needs a logged-in OpenCode; balance required).
- The probe shapes in this document came from OpenCode 2.3.2: `initialize`
  answers `protocolVersion: 1` and advertises `loadSession: true`;
  `session/load` returns `configOptions` and no `sessionId`; a read tool
  reported `tool_call` with `status: pending`, then `tool_call_update` with
  `in_progress` and `completed` and the directory in `locations[0].path`.
