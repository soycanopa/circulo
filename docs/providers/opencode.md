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
   `sessionId` and `configOptions` (models, agents) that the pickers will use.
3. `session/prompt` with one text part. Updates stream as
   `session/update` notifications until the response arrives.
4. The process is killed when the turn ends.

## Mapping

| ACP `sessionUpdate` | Circulo `Part` |
|---|---|
| `agent_message_chunk` | `Text { delta }` |
| `agent_thought_chunk` | `Reasoning { delta }` |
| `tool_call`, `tool_call_update` | `Tool { name, detail, state }`, one row per `toolCallId` |

`detail` prefers the tool's first location path, then `rawInput.path`. A tool is
`Running` while pending or in progress, `Done` when completed or failed; a
failure appends `· failed` to the detail. Unknown update kinds are ignored, so
ACP can add them without breaking the driver.

## Limits, recorded on purpose

- **One process per turn.** No shared mutable process and no half-dead session.
  `session/load` (the agent advertises `loadSession: true`) is how a session will
  be resumed once sessions are persisted.
- **Permissions are cancelled.** There is no permission card in the protocol
  yet, so `session/request_permission` is answered as cancelled and the tool
  does not run. Reads still work: OpenCode's defaults allow them.
- **No model or agent selection.** `session/new` returns `configOptions`; wiring
  them into `session/set_config_option` is the picker slice.
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
