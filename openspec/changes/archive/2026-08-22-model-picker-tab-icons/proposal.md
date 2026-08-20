# model-picker-tab-icons

## Why

The previous change (`model-picker-provider-tabs`) added a vertical column of provider tabs to the model popover, but each tab showed the provider's name as a text label. With two providers the labels are redundant: the user has already learned to recognize the OpenCode and CommandCode marks from the Settings → Models panel, where the change replaced the text badge with a provider icon.

This change replaces the text label in the popover tabs with the provider icon. The active tab is identified by the accent surface; the count sits below the icon. The tab width shrinks from 96px to 64px because the icon is the only content on the label row.

## What Changes

| Area | Outcome |
| --- | --- |
| `crates/circulo-app/assets/icons/commandcode.svg` | NEW: brand-supplied SVG path, viewBox 0 0 11 11, rounded-square-with-plus |
| `crates/circulo-app/src/icons.rs` | `COMMANDCODE` now points to the new file (was `chevron-right.svg`) |
| `crates/circulo-app/src/context_menu.rs` | `model_picker_provider_tabs` drops the `String` label from the per-tab tuple; the function renders the provider icon and the count. Tab width constant shrinks to 64px. |
| `crates/circulo-app/src/composer/view.rs` | Caller no longer fetches the i18n badge string per tab. The popover is unchanged otherwise. |
| `crates/circulo-i18n/locales/en.json` | Drops the unused `composer.model.tab_count` key (the count is now `format!("{count}")` in code). |
| Specs | `composer-stream` "Model picker has provider tabs" updated to say "provider's icon" instead of "provider's name". |

## Capabilities

### Modified Capabilities

- `composer-stream`: popover tabs render the provider icon (no text).

### New Capabilities

(none)

## Impact

- **Crates**: `circulo-app` only. No protocol or daemon changes.
- **External API**: none.
- **Behavior**: the picker shows an OpenCode-logo tab and a CommandCode-mark tab; the active tab is filled with the accent surface; the count sits below. The semantic behavior is identical to the previous change.

## Non-goals

- An "All" tab. The user explicitly chose per-provider tabs.
- Customizing the icons per theme.
- Animations on tab switch.

## Open questions

(none)