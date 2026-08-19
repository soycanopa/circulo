# Tasks: multi-provider-foundation

## 1. Core

- [x] 1.1 Add `CommandCode` variant to `AgentType` in `circulo-core/src/session.rs`

## 2. Protocol

- [x] 2.1 Re-export `AgentType` from `circulo-core` in `circulo-protocol`
- [x] 2.2 Add `agent: Option<AgentType>` to `CreateSessionRequest`
- [x] 2.3 Add `agent: Option<AgentType>` to `PatchSessionRequest`
- [x] 2.4 Add `AgentDescriptor` type for `GET /v1/agents`
- [x] 2.5 Add `circulo-core` as a dep of `circulo-protocol` in `Cargo.toml`

## 3. Daemon — registry

- [x] 3.1 Add `AdapterRegistry` struct in `circulo-daemon/src/lib.rs` with `opencode: Arc<dyn AgentAdapter>`
- [x] 3.2 Replace `AppState.adapter: Arc<dyn AgentAdapter>` with `AppState.registry: AdapterRegistry`
- [x] 3.3 Implement `AdapterRegistry::build()` (OpenCode always; honors `CIRCULO_ADAPTER=fake`)
- [x] 3.4 Implement `AdapterRegistry::for_agent(agent) -> Option<Arc<dyn AgentAdapter>>`
- [x] 3.5 Implement `AdapterRegistry::list() -> Vec<AgentDescriptor>`
- [x] 3.6 Replace `select_adapter()` in `main.rs` with `AdapterRegistry::build()`

## 4. Daemon — HTTP

- [x] 4.1 Add `service_unavailable` variant to `ApiError` (HTTP 503) if not present
- [x] 4.2 `POST /v1/sessions` uses `body.agent.unwrap_or(AgentType::OpenCode)`
- [x] 4.3 `PATCH /v1/sessions/{id}` handles `agent` with post-send lock
- [x] 4.4 Add `GET /v1/agents` route
- [x] 4.5 Resolve adapter via `state.registry.for_agent(session.agent)?` in the `create_session` handler before calling `run_assistant_turn`

## 5. Persist

- [x] 5.1 Roundtrip test: session with `agent = CommandCode` serializes and deserializes correctly

## 6. App — client

- [x] 6.1 Add `DaemonClient::list_agents() -> Result<Vec<AgentDescriptor>, String>`
- [x] 6.2 Add `DaemonClient::create_session_with_agent(project_id, agent) -> Result<Session, String>`
- [x] 6.3 Add `DaemonClient::patch_session_agent(session_id, agent) -> Result<Session, String>`
- [x] 6.4 Keep existing `create_session` / `create_session_with_project` but have them default to `OpenCode`

## 7. App — state and UI

- [x] 7.1 Add `available_agents: Vec<AgentDescriptor>` to `AppShell`
- [x] 7.2 Add `pending_agent: Option<AgentType>` to `AppShell`
- [x] 7.3 Add `AppShell::refresh_available_agents(cx)` handler
- [x] 7.4 Add `AppShell::set_pending_agent(agent, cx)` handler
- [x] 7.5 Create `crates/circulo-app/src/composer/agent_selector.rs` with chip + popover
- [x] 7.6 Render the selector only when `available_agents.len() > 1`
- [x] 7.7 Disable the selector when `session.first_send_at.is_some()`
- [x] 7.8 Wire selection to a PATCH via `patch_session_agent`

## 8. i18n

- [x] 8.1 Add `composer.agent.label`, `opencode`, `command_code`, `unavailable`, `locked` to `en.json`

## 9. Specs

- [x] 9.1 `agent-adapter`: registry + dispatch requirement + 503 scenario
- [x] 9.2 `composer-stream`: AgentSelector visibility + lock requirement
- [x] 9.3 `local-daemon-api`: GET /v1/agents surface

## 10. OpenSpec artifacts

- [x] 10.1 `proposal.md`, `design.md`, `tasks.md` in `openspec/changes/2026-08-20-multi-provider-foundation/`

## 11. Verification

- [x] 11.1 `cargo check --workspace` clean
- [x] 11.2 `cargo test --workspace` — 153 tests pass + new persist test
- [x] 11.3 `python3 scripts/check-crate-boundaries.py` clean
- [x] 11.4 Manual: `curl /v1/agents` returns the OpenCode descriptor