# Tasks: commandcode-model-catalog

## 1. Adapter

- [x] 1.1 `ModelCatalogEntry` gains `agent: AgentType` with serde default `OpenCode`
- [x] 1.2 `circulo-adapter-opencode` populates `agent = OpenCode` on each entry
- [x] 1.3 `circulo-adapter-commandcode::catalog` static `MODEL_CATALOG` from the docs page
- [x] 1.4 `CommandCodeAdapter::list_models` returns the static list

## 2. Daemon

- [x] 2.1 `ModelCatalogCache` becomes per-provider (HashMap<AgentType, (Instant, Vec<...>)>)
- [x] 2.2 `get(&registry)` aggregates from every enabled provider, dedupes by `(agent, id)`
- [x] 2.3 `list_models` HTTP handler uses the new aggregator
- [x] 2.4 Daemon tests still pass (existing fixtures deserialize with the new field default)

## 3. App — client

- [x] 3.1 `DaemonClient::set_model_and_agent(session_id, model_id, agent) -> Result<Session, String>`

## 4. App — composer

- [x] 4.1 Composer model picker click resolves the entry's `agent`
- [x] 4.2 If `agent != session.agent`, call `set_model_and_agent`; otherwise keep `patch_session_composer`

## 5. App — Settings → Models

- [x] 5.1 Each row shows a provider badge ("OpenCode" / "Command Code") next to the existing provider_name

## 6. i18n

- [x] 6.1 `commandcode.badge`, `opencode.badge`, `settings.models.provider_label`, `settings.models.implicit_dispatch`, `composer.model.implicit_dispatch`, `settings.models.agent_label`

## 7. Specs

- [x] 7.1 `composer-stream`: model selection implies provider
- [x] 7.2 `local-daemon-api`: GET /v1/models aggregates + agent field per entry

## 8. OpenSpec artifacts

- [x] 8.1 `proposal.md`, `design.md`, `tasks.md` in `openspec/changes/2026-08-22-commandcode-model-catalog/`

## 9. Verification

- [x] 9.1 `cargo check --workspace` clean
- [x] 9.2 `cargo test --workspace` — 168 + new pass
- [x] 9.3 `python3 scripts/check-crate-boundaries.py` clean
- [x] 9.4 Manual: Settings → Models shows CommandCode entries; picking one dispatches to CommandCode