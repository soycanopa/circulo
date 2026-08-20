# Design: model-picker-provider-tabs

## D1 — Composer state

`crates/circulo-app/src/composer/view.rs`:

```rust
pub struct Composer { /* ...existing... */
    model_picker_tab: AgentType,  // new
}
```

`model_picker_tab` is set to the current session's `agent` when models refresh (in `Composer::update` or wherever the model list is replaced). If the model list contains no entries for a provider, the corresponding tab disappears automatically because we derive tabs from `self.models`.

## D2 — Tab builder

`crates/circulo-app/src/context_menu.rs`:

```rust
pub const MODEL_PICKER_WITH_TABS_WIDTH_PX: f32 = 360.0;
const MODEL_PICKER_TAB_WIDTH_PX: f32 = 96.0;
const MODEL_PICKER_TAB_PX: f32 = 8.0;
const MODEL_PICKER_TAB_PY_PX: f32 = 6.0;

/// Vertical column of provider tabs for the model picker. One row per
/// provider that has at least one model in the catalog. The active tab
/// gets the accent background; inactive tabs are transparent and gain a
/// hover background on mouse-over.
pub fn model_picker_provider_tabs(
    current: AgentType,
    tabs: &[(AgentType, String, usize)],  // (agent, label, count)
    on_select: impl Fn(AgentType, &mut Window, &mut gpui::App) + 'static + Clone,
) -> impl IntoElement { ... }
```

`tabs` is the slice `(agent, label, count)` per provider. The label comes from the i18n catalog (`opencode.badge` / `commandcode.badge`); the count is the number of models in `self.models` with that `agent`.

Render shape:

```
┌────────┐ ┌─────────────────────────────────┐
│  OC 26 │ │  claude-sonnet-5       (Cmd)  │
│  CC 56 │ │  deepseek-v4-flash     (Cmd)  │
│        │ │  ...                            │
└────────┘ └─────────────────────────────────┘
```

Each tab is `div().id(...).h(...)` with the agent label + count stacked. Click → `on_select(agent)`.

## D3 — Popover render

`crates/circulo-app/src/composer/view.rs` — the model popover becomes:

```rust
let popover = div()
    .w(px(MODEL_PICKER_WITH_TABS_WIDTH_PX))
    .flex()
    .flex_row()
    .gap(px(MODEL_MENU_PADDING_PX))
    .p(px(MODEL_MENU_PADDING_PX))
    .rounded_lg()
    .bg(BG_MAIN)
    .shadow_lg()
    .occlude()
    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
    .child(model_picker_provider_tabs(
        self.model_picker_tab,
        &tabs,
        cx.listener(|this, agent, _, cx| this.set_model_picker_tab(agent, cx)),
    ))
    .child(div().flex_1().flex_col().gap(px(MODEL_MENU_ROW_GAP_PX)).child(/* header */).children(/* filtered rows */));
```

The right column keeps the existing `menu_model_selector_header` ("Favorites / Edit" line) + the filtered list. The header is shared across tabs. The list is filtered by `model.agent == self.model_picker_tab`.

`tabs` is built by counting the agents in `self.models`:

```rust
let mut tabs: Vec<(AgentType, String, usize)> = Vec::new();
for &agent in AgentType::ALL.iter() {
    let count = self.models.iter().filter(|m| m.agent == agent).count();
    if count == 0 { continue; }
    let label = match agent {
        AgentType::OpenCode => self.catalog.get("opencode.badge").to_string(),
        AgentType::CommandCode => self.catalog.get("commandcode.badge").to_string(),
    };
    tabs.push((agent, label, count));
}
```

## D4 — Default tab on refresh

`Composer::update` (the field that receives `models`) is also where we reset the tab. If the active provider isn't present anymore, fall back to the first available tab:

```rust
if !self.models.iter().any(|m| m.agent == self.model_picker_tab) {
    self.model_picker_tab = self.models.first().map(|m| m.agent)
        .unwrap_or(circulo_core::AgentType::OpenCode);
}
```

This handles the case where the user disables the current provider: the picker falls back to whatever's left.

## D5 — i18n

Two new keys (the rest of the tab text uses the existing `opencode.badge` / `commandcode.badge` from `provider-toggles`):

```json
"composer.model.tab_label": "{agent}",
"composer.model.tab_count": "{count}"
```

The count format is inlined next to the label (`OpenCode 26`).

## D6 — Spec deltas

`openspec/specs/composer-stream/spec.md`:

> ### Requirement: Model picker has provider tabs
>
> The model popover MUST show a vertical column of provider tabs on the left, one per Circulo provider that has at least one model in the visible catalog. Each tab MUST display the provider's name and the count of models it owns. The right column MUST render only the models whose `agent` matches the active tab. The default tab is the session's current `agent`; if that provider has no models, the picker falls back to the first available tab.

Scenarios:

> #### Scenario: Tabs reflect the catalog
> - **GIVEN** the daemon's `/v1/models` returns 26 OpenCode + 56 CommandCode models
> - **WHEN** the user opens the model picker
> - **THEN** the popover shows two tabs ("OpenCode 26" and "Command Code 56")
> - **AND** the right column lists only the models whose `agent` matches the active tab

> #### Scenario: Switching tabs filters the list
> - **GIVEN** the picker is open with the OpenCode tab active
> - **WHEN** the user clicks the Command Code tab
> - **THEN** the right column updates to show only Command Code models
> - **AND** the tab visual state updates to reflect the active tab

> #### Scenario: Disabling a provider removes its tab
> - **GIVEN** the user disables Command Code in Settings → Providers
> - **WHEN** the user re-opens the model picker
> - **THEN** only the OpenCode tab is shown
> - **AND** the right column shows only OpenCode models

## D7 — Files

| File | Change |
| --- | --- |
| `crates/circulo-app/src/composer/view.rs` | New `model_picker_tab` field; new handler; popover render becomes two-column |
| `crates/circulo-app/src/context_menu.rs` | `model_picker_provider_tabs` builder + width constant |
| `crates/circulo-app/src/composer/mod.rs` | Re-export the new builder (if needed) |
| `crates/circulo-i18n/locales/en.json` | 2 new keys |
| `openspec/specs/composer-stream/spec.md` | New requirement + 3 scenarios |

## D8 — Verification

| Step | Expected |
| --- | --- |
| `cargo check --workspace` | 0 warnings |
| `cargo test --workspace` | 171 + new tests pass |
| `python3 scripts/check-crate-boundaries.py` | clean |
| Manual | Open Circulo; open the model picker; see two vertical tabs; switch between them; the right column filters; clicking a model picks it and dispatches to the right provider (existing flow). |

## D9 — Commit strategy

```
docs(openspec): add model-picker-provider-tabs change artifacts
docs(specs): model picker has provider tabs
chore(i18n): composer.model.tab_label and tab_count
feat(app): model popover shows provider tabs
```

4 commits.

## D10 — Manual pass

1. Open Circulo. Create a new session. Default agent: OpenCode.
2. Open the model picker. See two vertical tabs ("OpenCode 26", "Command Code 56"); the OpenCode tab is active and the right column lists 26 OpenCode models.
3. Click the Command Code tab. Right column lists 56 Command Code models.
4. Pick one. Send a query. The session's `agent` becomes `command_code` (existing flow from `commandcode-model-catalog`).
5. Disable Command Code in Settings → Providers. Re-open the model picker. Only the OpenCode tab is shown.