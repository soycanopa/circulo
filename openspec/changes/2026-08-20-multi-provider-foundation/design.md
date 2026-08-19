# Design: multi-provider-foundation

## D1 — `AgentType` extension

`crates/circulo-core/src/session.rs` already has:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentType {
    OpenCode,
}
```

Add `CommandCode`. The `snake_case` rename gives the on-disk / on-wire value `command_code`. `Display` and any `FromStr` helpers (if present) get a parallel addition; today there are none in core, so this is a one-line enum extension.

`Session` and all serial roundtrips stay backward compatible: existing rows in SQLite have `agent = "opencode"` (snake_case of the previous variant), which continues to deserialize to `AgentType::OpenCode`.

## D2 — Protocol additions

`crates/circulo-protocol/src/lib.rs` gets three new shapes:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentType {
    OpenCode,
    CommandCode,
}

pub struct AgentDescriptor {
    pub agent: AgentType,
    pub available: bool,
    pub version: Option<String>,
}

pub struct CreateSessionRequest {
    #[serde(default)]
    pub project_id: Option<Uuid>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub agent: Option<AgentType>,
}

pub struct PatchSessionRequest {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub project_id: Option<Option<Uuid>>,
    #[serde(default)]
    pub archive: Option<bool>,
    #[serde(default)]
    pub agent: Option<AgentType>,           // new
    #[serde(default)]
    pub composer_model_id: Option<String>,
    // ... unchanged
}
```

`re-export AgentType` from `circulo-core` so the protocol crate doesn't duplicate the enum. (Currently `circulo-protocol` does not import from `circulo-core`; we'll add the dependency. The reverse direction is not affected.)

`AgentDescriptor` is the response shape for `GET /v1/agents`; today the descriptor list always contains `OpenCode` only.

## D3 — `AdapterRegistry` in the daemon

`crates/circulo-daemon/src/lib.rs`:

```rust
pub struct AdapterRegistry {
    opencode: Arc<dyn AgentAdapter>,
    // commandcode: Option<Arc<dyn AgentAdapter>>,   // added in change B
}

impl AdapterRegistry {
    pub fn build() -> Self { ... }                  // replaces select_adapter()
    pub fn for_agent(&self, agent: AgentType) -> Option<Arc<dyn AgentAdapter>> { ... }
    pub fn list(&self) -> Vec<AgentDescriptor> { ... }
    pub fn opencode(&self) -> &Arc<dyn AgentAdapter> { &self.opencode }
}
```

`AppState` swaps `adapter: Arc<dyn AgentAdapter>` for `registry: AdapterRegistry`. The `with_store` helper stays the same; `run_assistant_turn` now takes `&AdapterRegistry` (or we look up the adapter once in the HTTP handler and pass the resolved `Arc<dyn AgentAdapter>`). The latter is smaller surgery: keep `run_assistant_turn`'s signature, resolve `state.registry.for_agent(session.agent)?` in `http::create_session`'s handler before calling it.

`build()` always registers `OpenCode` via `OpenCodeAdapter::from_env()`. It honors `CIRCULO_ADAPTER=fake` for the test path (`FakeAdapter` is also registered as `OpenCode` since today's behavior conflates them). Command Code is a no-op in this change — `for_agent(CommandCode)` returns `None`.

`list()` calls `probe()` on each registered adapter; OpenCode's `probe()` already returns `Available` or `Unavailable`, and `opencode_health()` provides a `version` string when available.

## D4 — `GET /v1/agents` route

`crates/circulo-daemon/src/http.rs`:

```rust
async fn list_agents(
    State(state): State<AppState>,
) -> Json<Vec<AgentDescriptor>> {
    Json(state.registry.list())
}
```

Wired into `router()` as `GET /v1/agents`. No auth, no params. Returns the list, sorted (registry order is stable: OpenCode first, then optionals).

## D5 — `POST /v1/sessions` honors `body.agent`

In the existing `create_session` handler:

```rust
let agent = body.agent.unwrap_or(AgentType::OpenCode);
```

Persist the chosen agent. If the adapter is not registered (only possible for `CommandCode` in this change), the session is still created — the 503 is returned later when the user tries to send a message. This is a deliberate choice: the user can pre-stage a Command Code session in the daemon even before the adapter lands, and the sidebar / history remain readable.

`PATCH /v1/sessions/{id}` for the `agent` field:

```rust
if let Some(agent) = body.agent {
    if session.first_send_at.is_some() {
        return Err(ApiError::conflict("Agent is locked after the first send.").into());
    }
    session.agent = agent;
}
```

Same lock pattern as `project_id`.

## D6 — Generate dispatch

`circulo-daemon/src/generate.rs::run_assistant_turn` keeps its current shape; the HTTP handler resolves the adapter first:

```rust
let adapter = state.registry.for_agent(session.agent)
    .ok_or_else(|| ApiError::service_unavailable(
        format!("Agent {} is not registered in this build.", session.agent)
    ))?;
```

`ApiError` gets a new variant `service_unavailable` mapping to HTTP 503. If absent, add a new mapping alongside the existing `not_found`, `internal`, etc.

This means `CommandCode` sessions created in this change return a 503 on first send with a clear copy; the daemon does not crash.

## D7 — `circulo_app` changes

### `DaemonClient`

New methods in `crates/circulo-app/src/client.rs`:

```rust
pub fn list_agents(&self) -> Result<Vec<circulo_protocol::AgentDescriptor>, String> { ... }
pub fn create_session_with_agent(&self, project_id: Option<Uuid>, agent: AgentType) -> Result<Session, String> { ... }
pub fn patch_session_agent(&self, session_id: Uuid, agent: AgentType) -> Result<Session, String> { ... }
```

The existing `create_session` and `create_session_with_project` get a third default-`OpenCode` arg, or are re-expressed through `create_session_with_agent(None, AgentType::OpenCode)`. The same call sites that already pass `Some(project_id)` get an extra `AgentType::OpenCode`.

### Removing hardcoded `OpenCode`

Audit of `AgentType::OpenCode` literals today:

| File | Purpose | Replacement |
| --- | --- | --- |
| `circulo-app/src/client.rs:514, 582` | Test fixtures | Keep (test-only) |
| `circulo-app/src/command_palette.rs:132` | Test fixture | Keep (test-only) |
| `circulo-app/src/composer/helpers.rs:119` | Test fixture | Keep (test-only) |
| `circulo-daemon/src/generate.rs:474, 605, 669` | Test fixtures | Keep (test-only) |
| `circulo-core/src/lib.rs:40` | Test fixture | Keep (test-only) |
| `circulo-persist/src/lib.rs:42` | Test fixture | Keep (test-only) |
| `circulo-daemon/src/http.rs:321` | Production: hardcoded in `create_session` | Use `body.agent.unwrap_or(OpenCode)` |

So **only one** production hardcode changes: `http.rs:321`. The test fixtures stay because they exist to test the `OpenCode` variant specifically.

### `AgentSelector`

A new file `crates/circulo-app/src/composer/agent_selector.rs`:

- Renders a chip (similar to the model selector in `composer/`).
- Lists `AgentDescriptor` from the cached `GET /v1/agents` result.
- Disabled state: after the session's `first_send_at` is set.
- Click expands a popover with the list of available agents.
- Selecting an agent triggers `AppShell::set_pending_session_agent(agent, cx)`; on send, the actual PATCH is fired.

State in `AppShell`:

- `available_agents: Vec<circulo_protocol::AgentDescriptor>` (cached, refreshed on session open and on Settings mount).
- `pending_agent: Option<AgentType>` for in-progress selection (parallel to `pending_rename_project`).

The selector is **only** rendered when there is more than one available agent. Today, only OpenCode is registered, so the selector is hidden. After change B, it shows up.

This is the safe path: no UI flicker, no broken selector with one entry, and we get to test the wiring in A before exposing it.

## D8 — i18n

Add to `crates/circulo-i18n/locales/en.json`:

```json
"composer.agent.label": "Agent",
"composer.agent.opencode": "OpenCode",
"composer.agent.command_code": "Command Code",
"composer.agent.unavailable": "Not available in this build",
"composer.agent.locked": "Locked after the first send"
```

The composer pulls from the catalog. Until B lands, the selector isn't rendered so the strings are unused; but adding them now keeps the change self-contained.

## D9 — Spec deltas

### `openspec/specs/agent-adapter/spec.md`

Add:

> ### Requirement: Registry dispatches by `AgentType`
>
> The daemon MUST hold a registry of `Arc<dyn AgentAdapter>` keyed by `AgentType`. The daemon MUST dispatch `generate` calls to the adapter registered for the session's `agent`. A session whose `agent` is not registered MUST receive a 503 with a human error message at send time, not a crash.
>
> #### Scenario: OpenCode session dispatches to OpenCode adapter
> - **GIVEN** a session with `agent = OpenCode` and a registered OpenCode adapter
> - **WHEN** the user sends a message
> - **THEN** the daemon calls the OpenCode adapter's `generate`

> #### Scenario: Unregistered agent returns 503
> - **GIVEN** a session with `agent = CommandCode` and no CommandCode adapter registered
> - **WHEN** the user sends a message
> - **THEN** the daemon returns 503 with a human error

### `openspec/specs/composer-stream/spec.md`

Add:

> ### Requirement: AgentSelector visible only when more than one agent is available
>
> The composer MUST render an `AgentSelector` only when `GET /v1/agents` reports more than one provider. The selector MUST list the available providers and dispatch a `PATCH /v1/sessions/{id}` on selection. The selector MUST be disabled once the session's `first_send_at` is set.

### `openspec/specs/local-daemon-api/spec.md`

Add to the API surface list:

> `GET /v1/agents` — returns a list of `AgentDescriptor { agent, available, version? }` describing providers currently registered in this daemon build.

## D10 — Files

| File | Change |
| --- | --- |
| `crates/circulo-core/src/session.rs` | +1 enum variant |
| `crates/circulo-protocol/src/lib.rs` | + `AgentType` re-export, `AgentDescriptor`, agent fields on requests |
| `crates/circulo-protocol/Cargo.toml` | + dep on `circulo-core` |
| `crates/circulo-daemon/src/lib.rs` | + `AdapterRegistry`, swap `AppState.adapter` |
| `crates/circulo-daemon/src/main.rs` | replace `select_adapter()` with `AdapterRegistry::build()` |
| `crates/circulo-daemon/src/http.rs` | + `GET /v1/agents`, agent in `create_session` / `patch_session`, dispatch lookup, `service_unavailable` mapping |
| `crates/circulo-persist/src/store.rs` | + roundtrip test for non-OpenCode agent |
| `crates/circulo-app/src/client.rs` | + `list_agents`, `create_session_with_agent`, `patch_session_agent` |
| `crates/circulo-app/src/shell.rs` | + `available_agents`, + `pending_agent`, + handler to refresh + set agent, + wire `AgentSelector` render |
| `crates/circulo-app/src/composer/agent_selector.rs` | NEW: chip + popover + handlers |
| `crates/circulo-app/src/composer/mod.rs` | + `agent_selector` |
| `crates/circulo-i18n/locales/en.json` | + 5 keys |
| `openspec/specs/agent-adapter/spec.md` | + registry + dispatch requirement |
| `openspec/specs/composer-stream/spec.md` | + AgentSelector requirement |
| `openspec/specs/local-daemon-api/spec.md` | + GET /v1/agents |

## D11 — Verification

| Step | Expected |
| --- | --- |
| `cargo check --workspace` | 0 warnings introduced |
| `cargo test --workspace` | 153 tests pass + 1 new persist roundtrip test |
| `python3 scripts/check-crate-boundaries.py` | clean |
| Manual: `curl http://127.0.0.1:7432/v1/agents` | returns `[{"agent":"opencode","available":true,"version":"..."}]` |

## D12 — Commit strategy

```
docs(openspec): add multi-provider-foundation change artifacts
docs(specs): GET /v1/agents route
docs(specs): AgentSelector visibility rule
docs(specs): registry dispatches by AgentType
feat(protocol): agent field on session requests and AgentDescriptor
feat(core): add CommandCode variant to AgentType
feat(daemon): AdapterRegistry replaces single adapter
feat(daemon): GET /v1/agents and dispatch by session agent
feat(daemon): create_session honors body.agent
feat(persist): roundtrip test for non-OpenCode agent
feat(app): client methods for agents
feat(app): AgentSelector with conditional render
chore(i18n): composer agent labels
```

12 commits. The first three are the OpenSpec artifacts; the next two are spec deltas that document already-shipped shape; the rest are the implementation.

## D13 — Out of scope (deferred to `commandcode-adapter`)

- `circulo-adapter-commandcode` crate.
- `AdapterRegistry::commandcode: Option<Arc<dyn AgentAdapter>>`.
- Auth (Command Code requires `cmd login`).
- CWD / session binding per provider.
- Per-agent model catalogs.