# commandcode-adapter

## Why

PRD §10 marks v0.3 as "Un proveedor más, según demanda". The previous change `multi-provider-foundation` introduced the registry + dispatch + `GET /v1/agents` surface and added `CommandCode` to `AgentType`, but the registry's `for_agent(CommandCode)` returns `None`. This change ships the actual `circulo-adapter-commandcode` crate so a daemon build that has the binary installed can serve sessions whose `agent = command_code`.

Command Code ([commandcode.ai](https://commandcode.ai)) is a coding agent with a CLI (`cmd` shipped as the npm package `command-code`). It runs in **headless mode** for non-interactive use: `cmd -p "query" --output-format json`. The JSON output is NDJSON with event frames plus a final result line. The Circulo adapter wraps one headless run as a single turn.

## What Changes

| Area | Outcome |
| --- | --- |
| New crate | `circulo-adapter-commandcode` (parallel to `circulo-adapter-opencode`) |
| `CommandCodeAdapter` | Implements `AgentAdapter` against the headless subprocess |
| Binary discovery | `COMMANDCODE_BIN` env var → `cmd` on `PATH` |
| Auth | `EXIT_AUTH_ERROR=3` → `AdapterError::unavailable(Unauthorized, ...)`; copy tells the user to run `cmd login` |
| Workspace | New crate added to `Cargo.toml` members |
| `AdapterRegistry` | Adds `commandcode: Option<Arc<dyn AgentAdapter>>`; `for_agent(CommandCode)` returns the registered adapter when present; `list()` reports the new provider |
| Settings → General | Adds a Command Code row sourced from `GET /v1/agents` |
| i18n | `settings.commandcode.title`, `unavailable`, `auth_required`, `install_hint` |
| Specs | `agent-adapter` gains a Command Code mapping requirement; `local-daemon-api` documents that `/v1/agents` may return more than one descriptor |

## Capabilities

### Modified Capabilities

- `agent-adapter`: a second concrete impl, registered alongside the OpenCode one.
- `local-daemon-api`: `GET /v1/agents` returns one descriptor per registered provider (already in `multi-provider-foundation`; this change proves the second case works end-to-end).

### New Capabilities

(none — `AgentType::CommandCode` was added in the previous change)

## Impact

- **Crates**: new `circulo-adapter-commandcode`. Touches: `circulo-daemon` (registry wiring), `circulo-app` (Settings → General UI), `circulo-i18n` (new keys).
- **External API**: no change. The shape of `/v1/agents` is the same; the second entry simply appears when the binary is present.
- **Behavior**:
  - Daemon build with the binary present: sessions with `agent = command_code` run end-to-end via `cmd -p` (NDJSON → `AdapterEvent`).
  - Daemon build without the binary: the descriptor shows `available = false`; sessions created with that agent return 503 on send (same as the foundation change).
  - Auth: `cmd -p` exit code `3` becomes an `AdapterError::unavailable(Unauthorized, ...)` whose human message is the locale string `settings.commandcode.auth_required`.
- **Persistence**: no migration; existing sessions with `agent = opencode` keep working.

## Non-goals

- A streaming SSE-style wire (Command Code's headless mode is per-turn, not a long-running server).
- `list_models` programmatic catalog (Command Code does not expose one in headless mode; the trait default `Ok(Vec::new())` applies).
- `delete_agent_session` cleanup (sessions live on disk by directory; no remote handle to delete).
- Per-session / per-user model pricing or credits (deferred to a follow-up).
- Editing existing sessions' agent after `first_send_at` (already handled by the foundation change's lock).

## Open questions

(none)