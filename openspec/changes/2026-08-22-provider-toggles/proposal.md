# provider-toggles

## Why

Once the registry has more than one registered provider (`multi-provider-foundation` and `commandcode-adapter`), users need a way to control which providers are available to them. Today every registered provider is always-on: it shows up in the AgentSelector, accepts new sessions, and stays registered. For a non-technical user, the right level of control is a per-provider toggle in Settings.

This change adds a **Settings → Providers** section with a per-row enable/disable toggle. The toggle persists per user in `UserPreferences`; the daemon reads the set on startup and honors it on every relevant code path.

## What Changes

| Area | Outcome |
| --- | --- |
| `circulo_core::UserPreferences` | New `disabled_agents: BTreeSet<AgentType>` (default empty) |
| `circulo_protocol::UserPreferencesBody` | Same field, with `#[serde(default)]` for backward compat |
| `AgentDescriptor` | New `enabled: bool` (default `true`) |
| `AdapterRegistry` | `build(prefs)` reads the disabled set; `list()` reports `enabled` per entry |
| `GET /v1/agents` | Reflects the disabled set: disabled entries stay in the list with `enabled = false` |
| `POST /v1/agents/{agent}/{enable,disable}` | New endpoints; returns `UserPreferencesBody`; guard prevents disabling the last enabled provider |
| `POST /v1/sessions` | Rejects with `422 Unprocessable Entity` and `ErrorCode::AgentDisabled` if `body.agent` is disabled |
| Migration | When a provider is disabled, all existing sessions with that agent are migrated to `OpenCode` in the same SQLite transaction |
| Settings | New `SettingsSection::Providers` (5th nav item); per-row toggle with confirmation dialog |
| AgentSelector | `available_agents` filtered by `enabled`; selector hides when only one remains |
| i18n | 10+ keys under `settings.providers.*` |

## Capabilities

### Modified Capabilities

- `agent-adapter`: registry honors user preferences for enabled/disabled providers; the descriptor exposes `enabled`.
- `local-daemon-api`: new toggle endpoints; new `ErrorCode::AgentDisabled`; new `UserPreferencesBody.disabled_agents`.
- `app-shell`: new `SettingsSection::Providers`; the section is the only place where a provider can be disabled.

### New Capabilities

(none — the toggle is a feature, not a new capability)

## Impact

- **Crates**: `circulo-core`, `circulo-protocol`, `circulo-daemon`, `circulo-persist`, `circulo-app`, `circulo-i18n`.
- **External API**:
  - `GET /v1/agents` adds `enabled` per entry.
  - `POST /v1/agents/{agent}/enable` and `POST /v1/agents/{agent}/disable` are new.
  - `POST /v1/sessions` may return 422 with a new error code.
  - `UserPreferencesBody` adds `disabled_agents`; clients that don't send the field default to empty.
- **Behavior**:
  - When a provider is disabled, the AgentSelector hides it; new session creation rejects it; existing sessions migrate to OpenCode.
  - When the user tries to disable the last enabled provider, the API returns 409 with a human message and the preference is unchanged.
  - Migration: a `UPDATE sessions SET agent = 'opencode' WHERE agent = ?` runs in the same SQLite transaction as the preference write. The `agent_session_id` is left intact (the binding becomes stale but the new send will start a fresh OpenCode session for it; the conversation history stays in Circulo).
- **Persistence**: no migration. The schema already stores `agent` as `TEXT`; `UserPreferences` is keyed by a single row.

## Non-goals

- Auto-re-enable when the binary reappears (the user's decision persists).
- Per-session provider override (always at the user level).
- Migrating the prior conversation context to the new provider (the local messages persist; the agent context is lost and the user is told so in the confirmation copy).
- Settings for advanced options (attach mode, auth, etc.) — future changes.
- Web/team-scoped preferences (still per-user).

## Open questions

(none)