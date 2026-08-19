# Tasks: cleanup-and-doc-sync

Review order follows the design sections. Each slice: investigate → edit → verify → (user commits).

## 1. Crate cleanup — `circulo-adapter-opencode`

- [x] 1.1 Confirm `next_event` is unused outside its definition and own tests — verified via grep
- [x] 1.2 Delete `Client::next_event` (only `next_event_with_activity` is wired)

## 2. Crate cleanup — `circulo-daemon`

- [x] 2.1 Confirm `run_turn` is unused outside its own tests — verified via grep
- [x] 2.2 Delete `pub fn run_turn`
- [x] 2.3 Remove unused imports `QuestionAnswer`, `QuestionResponse` in `generate.rs`
- [x] 2.4 Remove unused import `PersistError` in `generate.rs`

## 3. Crate cleanup — `circulo-persist`

- [x] 3.1 Remove unused imports `ComposerInteractionMode`, `ComposerPermissionMode` in `store.rs`

## 4. Crate cleanup — `circulo-app`

- [x] 4.1 Remove unused import `BG_SIDEBAR` in `parts.rs`
- [x] 4.2 Remove unused imports `prelude::FluentBuilder`, `TEXT` in `settings/projects.rs`
- [x] 4.3 Remove unused import `path as icon_path` in `ui/menu_chip.rs`
- [x] 4.4 Remove unused import `should_apply_post_transcript` in `shell.rs`
- [x] 4.5 Delete `ComposerTextLayout::container_height` in `composer/text_layout.rs`
- [x] 4.6 Drop unused `line_height` param in `ComposerTextLayout::cursor_position`
- [x] 4.7 Delete `fn label` helper in `shell.rs` (line ~3558)

## 5. Doc sync — `README.md`

- [x] 5.1 Update Status table: drop "Workspace scaffold only"; reflect MVP feature-complete
- [x] 5.2 Update Project Definition version (v0.6 → v0.8)

## 6. Doc sync — `docs/PRD.md`

- [x] 6.1 Close §12 item 3 (Settings = General + Projects + Archived + Models)
- [x] 6.2 Close §12 item 5 (sidebar ordering = `last_message_at DESC`, NULL last)

## 7. Doc sync — `docs/TRD.md`

- [x] 7.1 Close §15 items 1–9 with one-line justification per item, including macOS 14 Sonoma and bundle ID `app.circulo.client`

## 8. Doc sync — `docs/POST-MVP.md`

- [x] 8.1 Reflect MVP feature-complete; leave backlog as v0.2+ deferred work

## 9. Spec sync — `sessions-sidebar`

- [x] 9.1 Add explicit `last_message_at DESC` ordering requirement with NULL last + `created_at DESC` tiebreaker

## 10. Spec sync — `app-shell`

- [x] 10.1 Add requirement that Settings exposes General, Projects, Archived, Models in that order

## 11. Verification

- [x] 11.1 `cargo check --workspace` clean of our warnings
- [x] 11.2 `cargo test --workspace` — 128 tests pass
- [x] 11.3 `python3 scripts/check-crate-boundaries.py` clean
- [x] 11.4 No new TODO/FIXME/unimplemented introduced