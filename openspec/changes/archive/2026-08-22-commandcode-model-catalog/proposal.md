# commandcode-model-catalog

## Why

The `commandcode-adapter` change shipped `list_models() -> Ok(Vec::new())` as a placeholder, on the basis that CommandCode's CLI does not expose a programmatic model catalog. Live testing proves that gap: the Settings → Models panel shows only OpenCode models, so the user has no way to pick a CommandCode model and (per the no-explicit-provider-selector rule) no way to dispatch a CommandCode session.

The CommandCode docs publish a definitive model list at `https://commandcode.ai/docs/reference/cli/models` and offer `cmd --list-models` in the CLI to print the same list. This change hardcodes the docs list into the adapter and wires it through the daemon and the app so the user can pick a CommandCode model in Settings and the implicit provider rule takes over.

## What Changes

| Area | Outcome |
| --- | --- |
| `circulo-adapter::ModelCatalogEntry` | New `agent: AgentType` field (default `OpenCode` via serde) so the app can read which Circulo provider serves a model |
| `circulo-adapter-commandcode` | Static `MODEL_CATALOG` from the docs page; `list_models()` returns it |
| `circulo-adapter-opencode` | Adapter populates `agent = OpenCode` on every entry; backward-compatible default if the daemon sends old fixtures |
| `circulo-daemon` | `GET /v1/models` aggregates the catalog from every enabled provider. The OpenCode cache is still OpenCode-only; the CommandCode cache is per-adapter; the handler merges and de-duplicates by `(agent, id)` |
| `circulo-app::client` | New `set_model_and_agent(session_id, model_id, agent)` PATCH helper; existing `patch_session_composer` stays for variant-only edits |
| `circulo-app::shell` | Composer model picker resolves the picked model's `agent` and sends both `composer_model_id` and `agent` to the daemon |
| `circulo-app::settings::models` | Each model row shows a provider badge (OpenCode / Command Code) next to the toggle |
| `circulo-i18n` | 4-6 new keys for the provider badge and the implicit-dispatch copy |
| Specs | `composer-stream` clarifies that picking a model updates the session's `agent`; `local-daemon-api` clarifies that `/v1/models` aggregates |

## Capabilities

### Modified Capabilities

- `composer-stream`: model selection updates the session's `agent` to the model's provider. The user no longer needs a separate provider selector in the composer.
- `local-daemon-api`: `GET /v1/models` returns the union of every enabled provider's catalog, each entry tagged with `agent`.

### New Capabilities

(none — the surface is the same; the data flows through the existing routes)

## Impact

- **Crates touched**: `circulo-core` (no change), `circulo-adapter`, `circulo-adapter-opencode`, `circulo-adapter-commandcode`, `circulo-daemon`, `circulo-app`, `circulo-i18n`.
- **External API**: `ModelCatalogEntry` gains a new field; old clients that don't send it see the default (`OpenCode`). `PATCH /v1/sessions/{id}` already accepts `agent`; the client starts using it on model pick.
- **Behavior**:
  - Models panel shows CommandCode models with a "Command Code" badge.
  - Picking a CommandCode model in the composer sets `session.agent = command_code` and `composer_model_id = <the id>`.
  - The session continues from there. If the user has disabled CommandCode, the daemon rejects the PATCH with 422 and the UI surfaces the existing copy.
  - Existing OpenCode sessions are unaffected.
- **Persistence**: no migration.

## Non-goals

- A `cmd --list-models` live shell-out call (the docs list is authoritative; the CLI invocation would add an auth dependency for what is, in practice, a stable catalog).
- A separate provider toggle in the composer (the user explicitly opted out).
- Per-model enable/disable per provider (current `enabled_model_ids` stays global).
- Migrating the OpenCode catalog away from `GET /global/models` (it stays the source of truth for OpenCode).

## Open questions

(none)