# Design: commandcode-model-catalog

## D1 — `ModelCatalogEntry.agent`

`crates/circulo-adapter/src/lib.rs` adds a new field with a serde default that keeps the OpenCode path backward-compatible:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelCatalogEntry {
    pub id: String,
    pub name: String,
    pub provider_id: String,
    pub provider_name: String,
    #[serde(default = "default_model_agent")]
    pub agent: AgentType,
}

fn default_model_agent() -> AgentType { AgentType::OpenCode }
```

The OpenCode adapter populates `agent = OpenCode` explicitly (or relies on the default — both are fine). The CommandCode adapter populates `agent = CommandCode`. Old data on disk deserializes to `OpenCode` via the default.

## D2 — CommandCode static catalog

`crates/circulo-adapter-commandcode/src/catalog.rs` (new file) holds a `LazyLock<Vec<ModelCatalogEntry>>` mirroring `https://commandcode.ai/docs/reference/cli/models`. Each entry has:

- `id = "<company>/<model>"` (e.g. `claude-sonnet-5`, `deepseek/deepseek-v4-flash`).
- `name` = the docs display name.
- `provider_id` = the upstream company slug (e.g. `anthropic`, `deepseek`).
- `provider_name` = human-readable company.
- `agent` = `AgentType::CommandCode`.

`CommandCodeAdapter::list_models()` returns a clone of the static list. The `--list-models` CLI flag exists but shelling out to it would add an auth dependency for a stable catalog; the docs list is sufficient.

## D3 — Daemon per-provider model catalog

`crates/circulo-daemon/src/model_catalog_cache.rs` becomes per-provider. New shape:

```rust
struct ModelCatalogCache {
    ttl: Duration,
    inner: Mutex<HashMap<AgentType, (Instant, Vec<ModelCatalogEntry>)>>,
}

impl ModelCatalogCache {
    fn new(ttl: Duration) -> Self { ... }
    fn get(&self, registry: &AdapterRegistry) -> Result<Vec<ModelCatalogEntry>, AdapterError> {
        // iterate AgentType::ALL, skip disabled, fetch each, dedupe by (agent, id)
    }
    fn fetch(&self, agent: AgentType, adapter: &dyn AgentAdapter) -> Result<Vec<ModelCatalogEntry>, AdapterError> {
        // honor TTL; otherwise call adapter.list_models()
    }
}
```

The `list_models` HTTP handler calls `cache.get(&state.registry)`. Disabled providers are skipped, so a disabled CommandCode doesn't leak its catalog into the picker.

Tests in `tests/api.rs` that build a fake catalog get the same `ModelCatalogEntry` shape (with the new field defaulted to `OpenCode`).

## D4 — `set_model_and_agent` client method

`crates/circulo-app/src/client.rs` adds:

```rust
pub fn set_model_and_agent(
    &self,
    session_id: Uuid,
    model_id: String,
    agent: AgentType,
) -> Result<Session, String> {
    self.patch(&format!("/v1/sessions/{session_id}"), &PatchSessionRequest {
        title: None, project_id: None, archive: None,
        agent: Some(agent),
        composer_model_id: Some(model_id),
        composer_model_variant: None,
        composer_permission_mode: None,
        composer_interaction_mode: None,
    })
}
```

`patch_session_composer` (variant-only) is unchanged — still used when the user picks a reasoning effort on the same model.

## D5 — Composer picker uses both fields

`crates/circulo-app/src/composer/view.rs` model picker click handler:

1. Find the picked entry in `state.composer_models`.
2. If `entry.agent != session.agent`: call `set_model_and_agent(entry.id, entry.agent)`.
3. Otherwise (same provider): call `patch_session_composer(entry.id, variant)`.

The picker UI doesn't change; the data flow does.

## D6 — Settings → Models panel

`crates/circulo-app/src/settings/models.rs` adds a small provider badge per row. The badge text comes from the catalog entry's `agent`:

```rust
let badge = match entry.agent {
    AgentType::OpenCode => "OpenCode".to_string(),
    AgentType::CommandCode => catalog.get("settings.commandcode.title").to_string(),
};
```

The existing `provider_name` (e.g. "Anthropic") is already shown next to the name. The agent badge is a separate, smaller chip that says which Circulo provider serves the model.

## D7 — i18n

```json
"commandcode.badge": "Command Code",
"opencode.badge": "OpenCode",
"settings.models.provider_label": "Provider",
"settings.models.implicit_dispatch": "Selecting this model sets the session to {agent}.",
"composer.model.implicit_dispatch": "This model is served by {agent}.",
"settings.models.agent_label": "Served by"
```

The implicit-dispatch copy is shown next to the model in the picker when it differs from the current session's agent. The Settings panel can also surface it as a hint on the row.

## D8 — Spec deltas

### `openspec/specs/composer-stream/spec.md`

Append a new requirement:

> ### Requirement: Model selection implies provider
>
> When the user picks a model in the composer's model picker, the session's `agent` MUST be set to the entry's `agent` field. This is the user-facing way to switch providers in v0.3 (no separate provider selector). If the chosen provider is disabled, the daemon returns 422 and the UI surfaces the existing copy.

### `openspec/specs/local-daemon-api/spec.md`

Update the `GET /v1/models` requirement:

> `GET /v1/models` MUST return the union of model catalogs from every registered provider whose `enabled` flag is `true`. Each entry MUST carry the `agent` field so the client can dispatch implicitly when picking a model.

## D9 — Files

| File | Change |
| --- | --- |
| `crates/circulo-adapter/src/lib.rs` | `ModelCatalogEntry.agent` field |
| `crates/circulo-adapter-opencode/src/client.rs` | populate `agent` on each entry |
| `crates/circulo-adapter-commandcode/src/catalog.rs` | NEW: static `MODEL_CATALOG` |
| `crates/circulo-adapter-commandcode/src/lib.rs` | `list_models` returns the static list |
| `crates/circulo-daemon/src/model_catalog_cache.rs` | per-provider cache + aggregator |
| `crates/circulo-daemon/src/http.rs` | `list_models` handler uses the new cache shape |
| `crates/circulo-app/src/client.rs` | `set_model_and_agent` |
| `crates/circulo-app/src/composer/view.rs` (or models.rs) | picker click dispatches with both |
| `crates/circulo-app/src/settings/models.rs` | provider badge per row |
| `crates/circulo-i18n/locales/en.json` | 6 keys |
| `openspec/specs/composer-stream/spec.md` | implicit-dispatch rule |
| `openspec/specs/local-daemon-api/spec.md` | aggregator + agent field |

## D10 — Verification

| Step | Expected |
| --- | --- |
| `cargo check --workspace` | 0 warnings |
| `cargo test --workspace` | 168 + new tests pass |
| `python3 scripts/check-crate-boundaries.py` | clean |
| Manual | Open Circulo, Settings → Models: see CommandCode models with "Command Code" badge. Pick a Claude model; the session's agent becomes CommandCode; the next send uses CommandCode. If CommandCode is disabled in Providers, the daemon returns 422 and the UI surfaces the existing copy. |

## D11 — Commit strategy

13 commits, granular by file/scope:

```
docs(openspec): add commandcode-model-catalog change artifacts
docs(specs): implicit provider dispatch via model selection
docs(specs): GET /v1/models aggregates per-provider catalogs
chore(i18n): provider badge and implicit dispatch copy
feat(app): Models panel shows provider badge per row
feat(app): composer picker sends model_id and agent together
feat(app): DaemonClient::set_model_and_agent
feat(daemon): GET /v1/models aggregates from all enabled providers
feat(daemon): ModelCatalogCache becomes per-provider
feat(adapter-opencode): populate agent on every catalog entry
feat(adapter-commandcode): static model catalog
feat(adapter): ModelCatalogEntry carries the agent
```

## D12 — Manual pass

After the change lands:
1. Restart the daemon so it picks up the new aggregator.
2. Open Circulo, go to Settings → Models. Expect OpenCode rows (with "OpenCode" badge) AND CommandCode rows (with "Command Code" badge).
3. Create a new session; the composer model picker shows the union. Pick a CommandCode row.
4. The session's `agent` becomes `command_code`.
5. Send a query. The CommandCode subprocess spawns with the chosen `--model`.