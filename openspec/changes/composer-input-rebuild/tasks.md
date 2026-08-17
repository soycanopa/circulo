# Tasks: composer-input-rebuild

## 1. OpenSpec and module skeleton

- [x] 1.1 Create change artifacts (proposal, design, delta spec, tasks)

## 2. ComposerInput entity (slice 1)

- [x] 2.1 `ComposerInput` with `EntityInputHandler`, `Render`, actions, `init(cx)`
- [x] 2.2 `ComposerInputElement` with unconditional `handle_input` and caret paint
- [x] 2.3 Wire `init` from `main.rs`; remove `composer_input.rs` and shell IME hacks

## 3. Composer container (slice 2)

- [x] 3.1 `Composer` entity with toolbar (picker, agent, send)
- [x] 3.2 Shell owns `Entity<Composer>`; `Submit` triggers send flow

## 4. Product behavior (slice 3)

- [x] 4.1 Per-session draft save/restore on session switch
- [x] 4.2 `read_only` while generating; focus on session select
- [x] 4.3 Restore draft on POST error; PATCH project on picker select

## 5. Tests and manual (slice 4)

- [x] 5.1 Unit tests for input actions and composer helpers
- [x] 5.2 Rewrite typing tests for `ComposerInput` (helpers + manual GPUI harness deferred)
- [x] 5.3 `cargo test -p circulo-app --lib` passes; manual macOS click-type-send pass pending user
