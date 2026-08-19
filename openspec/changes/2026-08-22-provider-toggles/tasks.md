# Tasks: provider-toggles

## 1. Core

- [x] 1.1 Add `disabled_agents: BTreeSet<AgentType>` to `UserPreferences` (default empty)
- [x] 1.2 Add `AgentType::all() -> [Self; N]` helper

## 2. Protocol

- [x] 2.1 Add `disabled_agents: Vec<AgentType>` to `UserPreferencesBody` (with `#[serde(default)]`)
- [x] 2.2 Add `enabled: bool` to `AgentDescriptor` (default `true`)
- [x] 2.3 Add `ErrorCode::AgentDisabled`
- [x] 2.4 Add `ApiError::agent_disabled(agent)` and `ApiError::last_provider_enabled()`

## 3. Daemon — registry

- [x] 3.1 `AdapterRegistry.build(prefs: &UserPreferences)` reads disabled set
- [x] 3.2 Add `disabled: HashSet<AgentType>` field; `set_disabled`; `is_enabled`
- [x] 3.3 `for_agent` returns `None` for disabled agents
- [x] 3.4 `list()` reports `enabled` per entry
- [x] 3.5 `with_opencode` test constructor initializes `disabled = HashSet::new()`

## 4. Daemon — endpoints

- [x] 4.1 `POST /v1/agents/{agent}/enable` returns 204 + new `UserPreferencesBody`
- [x] 4.2 `POST /v1/agents/{agent}/disable` returns 204 + new `UserPreferencesBody` after migration
- [x] 4.3 Last-enabled guard: returns 409 if toggle would leave zero enabled
- [x] 4.4 `POST /v1/sessions` returns 422 with `ErrorCode::AgentDisabled` for disabled agents
- [x] 4.5 `AdapterRegistry::set_disabled` is called by the disable handler so the in-memory state matches the persisted prefs
- [x] 4.6 `main.rs` reads preferences from store at startup; `AdapterRegistry::build(&prefs)`

## 5. Persist

- [x] 5.1 `Store::migrate_sessions_to_agent(from, to) -> Result<usize>` updates `agent` and returns the row count
- [x] 5.2 Roundtrip test: create CommandCode session, migrate to OpenCode, assert count and post-state

## 6. App — client

- [x] 6.1 `DaemonClient::set_provider_enabled(agent, enabled) -> Result<UserPreferencesBody, String>` (POSTs to enable/disable endpoint)

## 7. App — state

- [x] 7.1 `AppShell::preferences: UserPreferences`
- [x] 7.2 `AppShell::pending_provider_toggle: Option<(AgentType, bool)>` (toggle awaiting confirm)
- [x] 7.3 `AppShell::providers_pending_session_count: HashMap<AgentType, usize>` (counts for the dialog)
- [x] 7.4 `AppShell::refresh_provider_session_counts(cx)` (counts via list_sessions)
- [x] 7.5 `AppShell::refresh_preferences(cx)` (loads from /v1/preferences)
- [x] 7.6 `AppShell::request_provider_toggle(agent, enabled, cx)` (shows the confirm strip on disable)
- [x] 7.7 `AppShell::confirm_provider_toggle(agent, enabled, cx)` (calls the client, updates state)

## 8. App — Settings nav

- [x] 8.1 `SettingsSection::Providers` variant + label_key + nav_id
- [x] 8.2 `SettingsSection::ALL` updated to 5 items
- [x] 8.3 `settings_main_column` routes to `providers_panel` for the new section
- [x] 8.4 `settings/providers.rs` panel: list of providers with toggle and confirm strip

## 9. App — composer

- [x] 9.1 `AppShell::visible_agents()` filters by `enabled`
- [x] 9.2 AgentSelector render uses `visible_agents().len() > 1`

## 10. i18n

- [x] 10.1 `settings.section.providers`, `settings.providers.title`, `settings.providers.subtitle`, `settings.providers.active`, `settings.providers.disabled`, `settings.providers.not_installed`, `settings.providers.disable`, `settings.providers.enable`, `settings.providers.last_enabled_guard`, `settings.providers.confirm_disable`, `settings.providers.confirm_action`, `settings.providers.cancel`

## 11. Specs

- [x] 11.1 `agent-adapter`: registry respects disabled + last-enabled guard + 422 on session create
- [x] 11.2 `local-daemon-api`: enable/disable endpoints + ErrorCode::AgentDisabled + UserPreferencesBody field
- [x] 11.3 `app-shell`: SettingsSection::Providers + toggle UI + confirm dialog

## 12. OpenSpec artifacts

- [x] 12.1 `proposal.md`, `design.md`, `tasks.md` in `openspec/changes/2026-08-22-provider-toggles/`

## 13. Verification

- [x] 13.1 `cargo check --workspace` clean
- [x] 13.2 `cargo test --workspace` — 166 + new tests pass
- [x] 13.3 `python3 scripts/check-crate-boundaries.py` clean
- [x] 13.4 Manual: toggle each provider, confirm guard, confirm migration