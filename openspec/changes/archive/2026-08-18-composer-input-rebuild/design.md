# Design: Composer input rebuild

## Decisions

### D1 — Dedicated `ComposerInput` entity

Text state (`content`, `selected_range`, `marked_range`, `last_layout`, `last_bounds`) lives in `Entity<ComposerInput>`. `AppShell` does not implement `EntityInputHandler`.

### D2 — GPUI actions + `key_context("ComposerInput")`

Keyboard editing uses bound actions (`Enter`, `Newline`, `Backspace`, `Paste`, …), not `on_key_down` character injection.

### D3 — IME via `ComposerInputElement`

`window.handle_input` runs every paint frame on the input bounds. `EntityInputHandler` is implemented on `ComposerInput`.

### D4 — `Composer` container

Toolbar (project picker, static agent label, send/generating) and `Entity<ComposerInput>` are composed in `Entity<Composer>`. Shell subscribes to `ComposerEvent::Submit`.

### D5 — Per-session drafts

`HashMap<Uuid, String>` in `Composer`. On session switch, save current input text and restore the target session draft.

### D6 — Send error retains draft

On POST failure, restore the submitted text into the input instead of leaving the composer empty.

### D7 — PATCH on picker select

When unlocked, choosing a project in the picker calls `set_session_project` immediately (FLOWS §5.1).

## Reference

Waku concepts only: dedicated input entity, action dispatch, event emission, composer card separation. Implementation follows GPUI `examples/input.rs` patterns (MIT).
