# Design: model-picker-tab-icons

## D1 — New SVG asset

`crates/circulo-app/assets/icons/commandcode.svg` is the brand-supplied path:

```xml
<svg class="size-4" width="11" height="11" viewBox="0 0 11 11" ...>
  <path d="M10.7266 ..." fill="#575757"/>  <!-- rounded square outline -->
  <path fill-rule="evenodd" d="M5.27692..." fill="#575757"/>  <!-- the same outline, even-odd -->
  <path d="M3.36462 ..." fill="#000000"/>  <!-- the plus inside -->
</svg>
```

11x11 viewBox. The outer shell is `#575757`; the plus is `#000000`. The icon scales to whatever pixel size the caller renders at; we use 16x16 in the tab.

## D2 — Icon path constant

`crates/circulo-app/src/icons.rs`:

```rust
pub const OPENCODE: &str = "icons/opencode.svg";
pub const COMMANDCODE: &str = "icons/commandcode.svg";  // was icons/chevron-right.svg
```

The old chevron was a placeholder; the new path is the brand mark.

## D3 — Tab builder

`crates/circulo-app/src/context_menu.rs`:

```rust
const MODEL_PICKER_TAB_WIDTH_PX: f32 = 64.0;
const MODEL_PICKER_TAB_ICON_PX: f32 = 16.0;

pub fn model_picker_provider_tabs(
    current: AgentType,
    tabs: Vec<(
        AgentType,
        usize,
        Box<dyn Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static>,
    )>,
) -> impl IntoElement {
    // For each (agent, count, on_click):
    //   - icon (16x16) at the top
    //   - count number below, xs muted
    //   - active tab: ACCENT surface bg, ACCENT text
    //   - inactive tab: TEXT_MUTED text, hover BG_HOVER
}
```

The `String` label is gone. The caller in `view.rs` no longer fetches `opencode.badge` / `commandcode.badge` for tabs. The tab width drops from 96 to 64 — there's no text row to fit.

The popover width (`MODEL_PICKER_WITH_TABS_WIDTH_PX = 360.0`) is unchanged. The right column gains ~32px of breathing room.

## D4 — Caller simplification

`crates/circulo-app/src/composer/view.rs`:

```rust
// before
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

// after
let mut tabs: Vec<(AgentType, usize)> = Vec::new();
for &agent in AgentType::ALL.iter() {
    let count = self.models.iter().filter(|m| m.agent == agent).count();
    if count == 0 { continue; }
    tabs.push((agent, count));
}
```

The `composer.model.tab_count` i18n key is no longer used and gets removed.

## D5 — Files

| File | Change |
| --- | --- |
| `crates/circulo-app/assets/icons/commandcode.svg` | NEW |
| `crates/circulo-app/src/icons.rs` | `COMMANDCODE` path constant points to the new SVG |
| `crates/circulo-app/src/context_menu.rs` | Tab width 64px; tab builder drops the `String` label; new `MODEL_PICKER_TAB_ICON_PX` constant |
| `crates/circulo-app/src/composer/view.rs` | Caller passes `(agent, count, on_click)` tuples; no i18n lookup per tab |
| `crates/circulo-i18n/locales/en.json` | Drops `composer.model.tab_count` |
| `openspec/specs/composer-stream/spec.md` | Tab wording updated: "icon" instead of "name" |

## D6 — Verification

| Step | Expected |
| --- | --- |
| `cargo check --workspace` | 0 warnings |
| `cargo test --workspace` | 171 + new pass |
| `python3 scripts/check-crate-boundaries.py` | clean |
| Manual | Open Circulo, open the model picker, see two tabs with icons (no text). The active tab is the session's agent; switching tabs filters the list as before. |

## D7 — Commit strategy

```
chore(i18n): drop unused composer.model.tab_count
docs(specs): model picker tabs render provider icon
feat(app): provider icon in model picker tabs
chore(assets): add CommandCode brand mark
docs(openspec): add model-picker-tab-icons change artifacts
```

5 commits.

## D8 — Manual pass

1. Open Circulo. Open the model picker. See two tabs (OpenCode logo + CommandCode mark) with the count below. No text labels.
2. Click the CommandCode tab. Right column updates; tab background highlights the active one.
3. Pick a model. The session's `agent` switches per the existing `commandcode-model-catalog` rule.