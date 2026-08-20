# model-picker-provider-tabs

## Why

The previous change (`commandcode-model-catalog`) brought the Command Code catalog into the picker, but the popover is a single flat list. With ~80 models split between OpenCode and Command Code, the user can't tell at a glance which provider a model belongs to, and scrolling through everything is friction.

This change adds a vertical column of provider tabs on the left side of the model popover. The right side keeps the current model list, filtered to the active tab. Each tab shows the provider name + the count of models under it. Tabs are derived from the models present in the picker; the default tab is the session's current `agent`.

## What Changes

| Area | Outcome |
| --- | --- |
| `crates/circulo-app/src/composer/view.rs` | `Composer.model_picker_tab: AgentType`; `set_model_picker_tab(agent, cx)`; popover render becomes a two-column flex (`tabs | list`); width grows to fit the tab column |
| `crates/circulo-app/src/context_menu.rs` | New `model_picker_provider_tabs(...)` builder; new `MODEL_PICKER_WITH_TABS_WIDTH_PX` constant; per-tab styling consistent with the chip language |
| `crates/circulo-app/src/composer/mod.rs` | Re-export the new tab builder |
| `crates/circulo-i18n/locales/en.json` | 2 keys: `composer.model.tab_label` (for the tooltip), `composer.model.tab_count` (count formatting) |
| Specs | `composer-stream` adds a "Provider tabs" requirement + scenarios |

## Capabilities

### Modified Capabilities

- `composer-stream`: model picker has provider tabs on the left, filtering the right side; default tab is the session's `agent`.

### New Capabilities

(none)

## Impact

- **Crates**: `circulo-app` only.
- **External API**: none. The provider tab is internal UI state.
- **Behavior**:
  - When the user opens the picker, the active tab is the session's current `agent`. If the session is `command_code`, the CommandCode tab is selected; the user sees only CommandCode models.
  - Clicking a tab filters the right column to that provider's models.
  - Picking a model from the filtered list dispatches via `set_model_and_agent` (existing change), so switching tabs and picking from another provider still routes correctly.
  - If a provider is disabled in Settings → Providers, the tab doesn't appear (the picker only shows models from enabled providers, so the tab list is derived from the visible models).
  - Tabs are derived from `self.models`, so the count updates as the catalog refreshes.

## Non-goals

- An "All" tab. The user explicitly asked for per-provider tabs.
- Drag-to-reorder, pinned providers, or any other customisation. The default is fine.
- Changing the tab when the user picks a model (the model pick already handles dispatch via `set_model_and_agent`).

## Open questions

(none)