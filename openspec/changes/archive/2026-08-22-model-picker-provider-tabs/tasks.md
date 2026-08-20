# Tasks: model-picker-provider-tabs

## 1. UI

- [x] 1.1 `model_picker_provider_tabs` builder in `context_menu.rs` (vertical column of tabs with hover/active styling)
- [x] 1.2 `MODEL_PICKER_WITH_TABS_WIDTH_PX` constant + tab width constants
- [x] 1.3 Re-export the new builder from `composer::mod` if needed

## 2. Composer state

- [x] 2.1 `Composer.model_picker_tab: AgentType` field; default to the session's current agent on `update`
- [x] 2.2 `set_model_picker_tab(agent, cx)` handler

## 3. Popover render

- [x] 3.1 Two-column flex (tabs | list) when the picker is open
- [x] 3.2 Derive tabs from `self.models` (one per `AgentType` with at least one model)
- [x] 3.3 Filter the right column by `self.model_picker_tab`
- [x] 3.4 Width grows to `MODEL_PICKER_WITH_TABS_WIDTH_PX`

## 4. i18n

- [x] 4.1 `composer.model.tab_label` (e.g. "{agent}")
- [x] 4.2 `composer.model.tab_count` (e.g. "{count}")

## 5. Specs

- [x] 5.1 `composer-stream`: model picker has provider tabs + 3 scenarios

## 6. OpenSpec artifacts

- [x] 6.1 `proposal.md`, `design.md`, `tasks.md` in `openspec/changes/2026-08-22-model-picker-provider-tabs/`

## 7. Verification

- [x] 7.1 `cargo check --workspace` clean
- [x] 7.2 `cargo test --workspace` — 171 + new pass
- [x] 7.3 `python3 scripts/check-crate-boundaries.py` clean
- [x] 7.4 Manual: tabs visible, switch filters, disabled provider tab disappears