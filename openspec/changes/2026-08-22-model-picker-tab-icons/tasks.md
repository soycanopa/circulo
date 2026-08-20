# Tasks: model-picker-tab-icons

## 1. Asset

- [x] 1.1 Save `crates/circulo-app/assets/icons/commandcode.svg` from the brand-supplied path

## 2. Icon path

- [x] 2.1 Update `COMMANDCODE` constant in `icons.rs` to point to the new file

## 3. Tab builder

- [x] 3.1 `model_picker_provider_tabs` drops the `String` label; renders icon + count
- [x] 3.2 Tab width constant shrinks to 64px
- [x] 3.3 New `MODEL_PICKER_TAB_ICON_PX` constant (16px)

## 4. Caller

- [x] 4.1 `view.rs` passes `(AgentType, usize, on_click)` tuples to the tab builder
- [x] 4.2 No more i18n lookup per tab

## 5. i18n

- [x] 5.1 Drop `composer.model.tab_count` (no longer used)

## 6. Specs

- [x] 6.1 `composer-stream`: tab wording updated to "provider's icon" (no "name")

## 7. OpenSpec artifacts

- [x] 7.1 `proposal.md`, `design.md`, `tasks.md` in `openspec/changes/2026-08-22-model-picker-tab-icons/`

## 8. Verification

- [x] 8.1 `cargo check --workspace` clean
- [x] 8.2 `cargo test --workspace` — 171 + new pass
- [x] 8.3 `python3 scripts/check-crate-boundaries.py` clean
- [x] 8.4 Manual: tabs render provider icons, click filters, count sits below