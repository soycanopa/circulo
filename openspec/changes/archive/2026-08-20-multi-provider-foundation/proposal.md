# multi-provider-foundation

## Why

PRD §10 marks v0.3 as "Un proveedor más, según demanda". Today the codebase is hardwired to OpenCode:

- `circulo_core::AgentType` has a single variant.
- `POST /v1/sessions` ignores any agent choice and stores `AgentType::OpenCode` (`circulo-daemon/src/http.rs:321`).
- `AppState` holds one `Arc<dyn AgentAdapter>` chosen at startup via `CIRCULO_ADAPTER` env (`circulo-daemon/src/main.rs:25`).
- `circulo_app` hardcodes `AgentType::OpenCode` in 5+ places.
- The composer's `AgentSelector` does not exist as a UI surface; the field is unused in the model.

Before a second provider (e.g. Command Code) can land, the app, protocol, daemon, and core need an extensible multi-provider surface. This change is that surface. It does **not** ship the second provider; that is a separate change.

## What Changes

| Area | Outcome |
| --- | --- |
| `circulo_core` | `AgentType` adds a `CommandCode` variant. Serde renames follow `snake_case` (so the on-disk value is `command_code`). |
| `circulo_protocol` | `CreateSessionRequest { agent: Option<AgentType> }` and `PatchSessionRequest { agent: Option<AgentType> }`. New `AgentDescriptor { agent, available, version? }` returned by `GET /v1/agents`. |
| `circolo_daemon` | New `AdapterRegistry` holds `Arc<dyn AgentAdapter>` per available provider. Replaces the single `AppState.adapter`. `build_registry()` auto-detects OpenCode (always) and skips providers whose binary / setup is missing. New `GET /v1/agents` route. `POST /v1/sessions` honors `body.agent`, defaulting to `OpenCode`. `generate` dispatches by `session.agent`; a session whose adapter is not registered returns a 503 with a human message. |
| `circulo_persist` | Roundtrip test confirms a non-OpenCode agent serializes through the existing `TEXT` column without migration. |
| `circulo_app` | `DaemonClient` gets `list_agents`, `create_session_with_agent`, `patch_session_agent`. The 5+ `AgentType::OpenCode` literals are removed: session creation reads the user-chosen agent, command palette / composer helpers / refresh paths use the session's actual agent. New `AgentSelector` in the composer: dropdown of providers from `GET /v1/agents`; locked after the first send (parallel to `ProjectFolderSelector`). |
| i18n | `composer.agent.label`, `composer.agent.opencode`, `composer.agent.command_code`, `composer.agent.unavailable`, `composer.agent.locked`. |
| Specs | `agent-adapter` gets a registry + dispatch requirement. `composer-stream` gets an `AgentSelector` requirement (selector visible pre-send, locked post-send). |

## Capabilities

### Modified Capabilities

- `agent-adapter`: registry with one or more `Arc<dyn AgentAdapter>`; dispatch by `AgentType`.
- `composer-stream`: explicit `AgentSelector` UX (pre-send choice, post-send lock).
- `local-daemon-api`: `GET /v1/agents` route.
- `circulo-protocol`: `agent` field on `CreateSessionRequest` and `PatchSessionRequest`.
- `domain-model`: `AgentType` enum adds `CommandCode`.

### New Capabilities

(none)

## Impact

- **Crates touched**: `circulo-core`, `circulo-protocol`, `circulo-daemon`, `circulo-persist`, `circulo-app`, `circulo-i18n`. No new crate.
- **External API**: new `GET /v1/agents`. `POST /v1/sessions` and `PATCH /v1/sessions/{id}` accept an optional `agent` field; absence keeps prior behavior.
- **Behavior**:
  - Existing sessions with `agent = "opencode"` continue to work unchanged.
  - New sessions default to `OpenCode` if the request body omits `agent`.
  - In this change, `GET /v1/agents` only reports OpenCode (Command Code returns 503 if a session targets it). A follow-up change adds the Command Code adapter.
- **Persistence**: no migration; the schema already stores `agent` as `TEXT`.

## Non-goals

- Adding the Command Code adapter (next change: `commandcode-adapter`).
- Editing the trait's `opencode_health()` method (left as-is; OpenCode-specific).
- Changing sessions in flight or migrating existing data.
- Attach mode (deferred per `docs/POST-MVP.md` §2).
- Per-agent settings.
- Caching the agent list (the daemon re-probes on every request).

## Open questions

(none)