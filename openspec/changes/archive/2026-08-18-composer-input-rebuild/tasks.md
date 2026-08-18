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

## 6. Multiline growth, scroll, and expand

- [x] 6.1 Wrap long lines with `shape_text`; grow height up to 5 visual lines
- [x] 6.2 Scroll inside the input when content exceeds the visible cap
- [x] 6.3 Expand icon when >5 lines; expanded mode shows up to 10 lines
- [ ] 6.4 Manual macOS pass: long line wrap, scroll, expand/collapse

## 7. Composer footer (project select + work mode)

- [x] 7.1 Project select below composer card, left-aligned
- [x] 7.2 Local / Remote work-mode indicator beside the select
- [x] 7.3 Select lists projects, Without Folder, and Open project… (folder picker)
- [ ] 7.4 Manual pass: footer layout, open project flow, locked state after first send
