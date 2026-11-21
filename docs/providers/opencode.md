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
2. `session/new` with `cwd` and an empty `mcpServers` list. The answer carries a
   `sessionId` and `configOptions` (models, effort, modes) behind the pickers.
   The stored choices are re-applied with `session/set_config_option`, because
   a set lasts only for the session that received it.
3. `session/prompt` with one text part. Updates stream as
   `session/update` notifications until the response arrives.
4. The process is killed when the turn ends.

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

- **One process per turn.** No shared mutable process and no half-dead session.
  `session/load` (the agent advertises `loadSession: true`) is how a session will
  be resumed once sessions are persisted.
- **Permissions ask the user.** `session/request_permission` becomes a card and
  the turn blocks on the answer; a client that leaves reads as cancelled.
  Reads still work without asking: OpenCode's defaults allow them.
- **No per-turn memory.** A set lasts for the agent session that received it: a
  fresh `session/new` resets `effort` to `default` (probed on 2.3.2). The
  driver stores the user's choices and re-applies them after every
  `session/new`, so a switch survives the per-turn process. A stale choice the
  agent refuses is dropped.
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
- `cargo test -p circulo-provider-opencode -- --ignored` runs a real turn
  (needs a logged-in OpenCode and spends a model call).
- The probe shapes in this document came from OpenCode 2.3.2: `initialize`
  answers `protocolVersion: 1`; a read tool reported `tool_call` with
  `status: pending`, then `tool_call_update` with `in_progress` and `completed`
  and the directory in `locations[0].path`.
