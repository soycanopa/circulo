# Design: provider-toggles

## D1 — `UserPreferences.disabled_agents`

`crates/circulo-core/src/...`:

```rust
use std::collections::BTreeSet;

pub struct UserPreferences {
    pub enabled_model_ids: Vec<String>,
    pub disabled_agents: BTreeSet<AgentType>,
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            enabled_model_ids: Vec::new(),
            disabled_agents: BTreeSet::new(),
        }
    }
}
```

`BTreeSet` ensures deterministic iteration order for tests and JSON output.

`circulo-protocol`:

```rust
pub struct UserPreferencesBody {
    #[serde(default)]
    pub enabled_model_ids: Vec<String>,
    #[serde(default)]
    pub disabled_agents: Vec<circulo_core::AgentType>,
}
```

`Vec` on the wire (clearer JSON), converted to `BTreeSet` in core.

## D2 — `AgentDescriptor.enabled`

Add `enabled: bool` (default `true`) to `AgentDescriptor`. The wire shape:

```rust
pub struct AgentDescriptor {
    pub agent: AgentType,
    pub available: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    pub enabled: bool,  // new
}
```

The two flags are orthogonal:
- `available` = binary present + auth ok (technical state).
- `enabled` = user preference (toggle).

`GET /v1/agents` reports both. The Settings UI combines them for copy.

## D3 — `AdapterRegistry` reads preferences

`crates/circulo-daemon/src/adapter_registry.rs`:

```rust
pub struct AdapterRegistry {
    opencode: Arc<dyn AgentAdapter>,
    commandcode: Option<Arc<dyn AgentAdapter>>,
    disabled: HashSet<AgentType>,
}

impl AdapterRegistry {
    pub fn build(prefs: &UserPreferences) -> Self { ... }
    pub fn with_opencode(opencode: Arc<dyn AgentAdapter>) -> Self {
        Self { opencode, commandcode: None, disabled: HashSet::new() }
    }
    pub fn set_disabled(&mut self, agent: AgentType, disabled: bool) {
        if disabled { self.disabled.insert(agent); } else { self.disabled.remove(&agent); }
    }
    pub fn is_enabled(&self, agent: AgentType) -> bool { !self.disabled.contains(&agent) }
    pub fn for_agent(&self, agent: AgentType) -> Option<Arc<dyn AgentAdapter>> {
        if !self.is_enabled(agent) { return None; }
        match agent {
            AgentType::OpenCode => Some(Arc::clone(&self.opencode)),
            AgentType::CommandCode => self.commandcode.as_ref().map(Arc::clone),
        }
    }
    pub fn list(&self) -> Vec<AgentDescriptor> {
        // each entry gets `enabled = self.is_enabled(agent)`
    }
}
```

`for_agent` now returns `None` for disabled agents — the same path the foundation change used for "agent not registered". The 503 + human error from the foundation is what the user sees.

## D4 — New endpoints

```rust
async fn disable_agent(
    State(state): State<AppState>,
    Path(agent): Path<AgentType>,
) -> Result<Json<UserPreferencesBody>, HttpError> {
    // 1. Read current prefs
    // 2. If already disabled, return current prefs (idempotent)
    // 3. Apply temporarily: would other providers remain enabled?
    //    (count currently-enabled minus the candidate; must be >= 1)
    //    If 0 -> return 409 "At least one provider must stay enabled."
    // 4. Migrate sessions in same transaction as preference write
    // 5. Reload registry on AppState (so for_agent reflects the change immediately)
    // 6. Return updated UserPreferencesBody
}

async fn enable_agent(
    State(state): State<AppState>,
    Path(agent): Path<AgentType>,
) -> Result<Json<UserPreferencesBody>, HttpError> { ... }
```

The endpoints are atomic: preference write + session migration + registry reload happen together.

The guard runs **before** any side effect:

```rust
let mut next = state.preferences.clone();
next.disabled_agents.insert(agent);
let would_remain = AgentType::all().iter().filter(|a| !next.disabled_agents.contains(a)).count();
if would_remain == 0 {
    return Err(HttpError::from(ApiError::last_provider_enabled()));
}
```

`AgentType::all()` is a new helper returning the full set of variants. Needed for the guard.

## D5 — `POST /v1/sessions` rejects disabled agents

`crates/circulo-daemon/src/http.rs::create_session`:

```rust
let agent = body.agent.unwrap_or(AgentType::OpenCode);
if !state.registry.is_enabled(agent) {
    return Err(HttpError::from(ApiError::agent_disabled(agent)));
}
```

`ErrorCode::AgentDisabled` is a new variant; HTTP status `422 Unprocessable Entity`. The handler mapping:

```rust
ErrorCode::AgentDisabled => StatusCode::UNPROCESSABLE_ENTITY,
```

## D6 — Migration in persist

`crates/circulo-persist/src/store.rs`:

```rust
pub fn migrate_sessions_to_agent(
    &self,
    from: AgentType,
    to: AgentType,
) -> Result<usize, PersistError> {
    let updated = self.conn.execute(
        "UPDATE sessions SET agent = ?1, updated_at = ?2 WHERE agent = ?3",
        params![enum_to_db(&to)?, format_time(OffsetDateTime::now_utc())?, enum_to_db(&from)?],
    )?;
    Ok(updated)
}
```

Returns the row count so the daemon can include it in the response (for the UI's confirmation count, but with the simpler UX we just always migrate).

Test: `non_opencode_session_roundtrip_after_migration` creates CommandCode sessions, migrates them, asserts they're now OpenCode and the count is correct.

## D7 — Settings UI

`crates/circulo-app/src/settings/mod.rs`:

```rust
pub enum SettingsSection {
    General,
    Projects,
    Archived,
    Providers,  // new
    Models,
}

impl SettingsSection {
    pub const ALL: [Self; 5] = [Self::General, Self::Projects, Self::Archived, Self::Providers, Self::Models];
    pub fn label_key(self) -> &'static str { ... }  // adds "settings.section.providers"
    pub fn nav_id(self) -> &'static str { ... }  // adds "settings-nav-providers"
}
```

`crates/circulo-app/src/settings/providers.rs`:

```rust
pub fn providers_panel(
    descriptors: &[AgentDescriptor],
    pending: Option<AgentType>,
    catalog: &Catalog,
    cx: &mut Context<AppShell>,
) -> impl IntoElement { ... }
```

Each row: provider name, status badge (Active / Disabled / Not installed), toggle button. Disabled rows show a "Re-enable" button. The toggle triggers a confirm dialog when disabling a provider with active sessions (count comes from `GET /v1/sessions?agent=<agent>` — but for simplicity, we let the daemon's auto-migration do the work and the UI just shows the dialog with "Disable anyway").

`AppShell`:
- `preferences: UserPreferences` (mirror).
- `pending_provider_toggle: Option<(AgentType, bool)>` — the toggle request awaiting confirmation. When set, an inline confirm strip appears with "Disable CommandCode? Existing sessions will move to OpenCode." plus Confirm / Cancel.
- `providers_pending_session_count: HashMap<AgentType, usize>` — counts existing sessions per agent, fetched when entering the Providers section.

`AppShell::refresh_provider_session_counts`:
- Calls `list_sessions()` and groups by `agent`, populating `providers_pending_session_count`.
- Called on entering the Providers section.

`AppShell::request_provider_toggle(agent, enabled, cx)`:
- If disabling: show the confirm strip.
- If enabling: skip the strip, call `set_provider_enabled` directly.

`AppShell::confirm_provider_toggle(agent, enabled, cx)`:
- Calls `DaemonClient::set_provider_enabled(agent, enabled)`.
- On success: updates `preferences` and `pending_provider_toggle`, refreshes sessions.

## D8 — Composer selector

`AppShell::available_agents` becomes:

```rust
fn visible_agents(&self) -> Vec<AgentDescriptor> {
    self.available_agents.iter()
        .filter(|d| d.enabled)
        .cloned()
        .collect()
}
```

The composer renders the selector when `visible_agents().len() > 1`. When the user disables a provider, the selector updates within the same refresh cycle.

## D9 — i18n

```json
"settings.section.providers": "Providers"
"settings.providers.title": "Providers"
"settings.providers.subtitle": "Choose which AI providers you can use in Circulo. Disabled providers stay registered but cannot be selected for new sessions."
"settings.providers.active": "Active"
"settings.providers.disabled": "Disabled"
"settings.providers.not_installed": "Not installed"
"settings.providers.disable": "Disable"
"settings.providers.enable": "Enable"
"settings.providers.last_enabled_guard": "At least one provider must stay enabled."
"settings.providers.confirm_disable": "Disable {provider}? {count, plural, one{# session} other{# sessions}} currently using {provider} will move to {default_provider}. Conversation history stays in Circulo; the new agent won't see the prior context."
"settings.providers.confirm_action": "Disable"
"settings.providers.cancel": "Cancel"
```

`Catalog::get` returns the raw string. The confirm dialog substitutes `{provider}`, `{count}`, `{default_provider}` via `format!` at the call site. (No ICU plural support in our `Catalog`; the "plural" wording is fixed by the `one`/`other` branches of the source string.)

## D10 — Spec deltas

### `openspec/specs/agent-adapter/spec.md`

Add a requirement and three scenarios:

> ### Requirement: Registry respects user-enabled providers
>
> The `AdapterRegistry` MUST honor `UserPreferences.disabled_agents` on every dispatch. A disabled provider MUST NOT be returned by `for_agent` and MUST report `enabled = false` in the descriptor. The adapter instance MAY still be constructed (so re-enabling is a state change, not a re-build), but it MUST NOT be invoked.

Scenarios:
- Disabling a provider with no other providers returns 409 and the state is unchanged.
- Disabling a provider with other providers returns the updated `UserPreferencesBody` and migrates the provider's existing sessions to OpenCode.
- `POST /v1/sessions` with a disabled agent returns 422 with `ErrorCode::AgentDisabled`.

### `openspec/specs/local-daemon-api/spec.md`

Add:
- `POST /v1/agents/{agent}/enable` and `POST /v1/agents/{agent}/disable` to the API surface.
- New `ErrorCode::AgentDisabled` returning 422.
- `UserPreferencesBody.disabled_agents` field.

### `openspec/specs/app-shell/spec.md`

Add:
- A fifth section `SettingsSection::Providers` with toggle UI and confirm dialog.
- The confirm dialog copy must surface the migration consequence.

## D11 — Files

| File | Change |
| --- | --- |
| `crates/circulo-core/src/...` | `UserPreferences.disabled_agents`; `AgentType::all()` helper |
| `crates/circulo-protocol/src/lib.rs` | `UserPreferencesBody.disabled_agents`; `AgentDescriptor.enabled`; `ErrorCode::AgentDisabled`; `ApiError::agent_disabled(...)`; `ApiError::last_provider_enabled(...)` |
| `crates/circulo-daemon/src/adapter_registry.rs` | `build(prefs)`; `disabled` field; `set_disabled`; `is_enabled`; updated `for_agent` / `list` |
| `crates/circulo-daemon/src/http.rs` | New handlers `enable_agent` / `disable_agent`; `create_session` rejects disabled; new routes |
| `crates/circulo-daemon/src/main.rs` | `AdapterRegistry::build(&prefs)` — load preferences from store at startup |
| `crates/circulo-persist/src/store.rs` | `migrate_sessions_to_agent(from, to)` |
| `crates/circulo-app/src/client.rs` | `set_provider_enabled(agent, enabled) -> Result<UserPreferencesBody, String>` |
| `crates/circulo-app/src/settings/mod.rs` | `SettingsSection::Providers`; `ALL` array extended |
| `crates/circulo-app/src/settings/providers.rs` | NEW panel |
| `crates/circulo-app/src/shell.rs` | `preferences`, `pending_provider_toggle`, `providers_pending_session_count`, handlers, settings nav |
| `crates/circulo-app/src/composer/...` | Use `visible_agents()` (filter by `enabled`) |
| `crates/circulo-i18n/locales/en.json` | + 10 keys |
| `openspec/specs/agent-adapter/spec.md` | toggle requirement + scenarios |
| `openspec/specs/local-daemon-api/spec.md` | endpoints + error code |
| `openspec/specs/app-shell/spec.md` | section + dialog |

## D12 — Verification

| Step | Expected |
| --- | --- |
| `cargo check --workspace` | 0 warnings introduced |
| `cargo test --workspace` | 166 + new migration + handler tests pass |
| `python3 scripts/check-crate-boundaries.py` | clean |
| Manual | Disable CommandCode → confirm dialog → existing CommandCode sessions migrate → AgentSelector no muestra CommandCode; nueva sesión con `agent = command_code` → 422 con copy humano; re-enable → AgentSelector vuelve a mostrar; toggle del último enabled → 409 + copy guard |

## D13 — Commit strategy

```
docs(openspec): add provider-toggles change artifacts
docs(specs): settings providers section and toggle behavior
docs(specs): provider enable/disable endpoints and disabled-agent rejection
docs(specs): agent registry respects user preferences
chore(i18n): settings.providers.* keys
feat(app): composer filters agents by enabled
feat(app): confirm dialog and migrate handler in shell
feat(app): SettingsSection::Providers and providers panel
feat(app): DaemonClient::set_provider_enabled
feat(daemon): reject session create with disabled agent
feat(persist): migrate_sessions_to_agent
feat(daemon): endpoints enable/disable + last-enabled guard
feat(daemon): AdapterRegistry honors disabled_agents
feat(protocol): UserPreferencesBody and AgentDescriptor enabled field
feat(core): UserPreferences.disabled_agents
```

15 commits. Some specs in there are debatable — could merge into 2 commits. Keeping them granular so the reviewer can approve the API surface separately from the migration behavior.

## D14 — Manual pass

Per AGENTS.md §1.8, the user runs:
1. `GET /v1/agents` shows both providers with `enabled = true`.
2. Settings → Providers → Disable CommandCode → confirm dialog with session count.
3. Existing CommandCode sessions show up under OpenCode in the sidebar.
4. AgentSelector no muestra CommandCode.
5. `POST /v1/sessions` with `agent = command_code` returns 422 (verify with curl).
6. Re-enable CommandCode → AgentSelector shows it again; new sessions can be created.