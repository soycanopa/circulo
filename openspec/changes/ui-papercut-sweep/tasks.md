# Tasks: ui-papercut-sweep

## 1. Non-blocking session activation

- [x] 1.1 `activate_session` (crates/circulo-app/src/shell.rs): clear `messages`, set `messages_loading = true`, remove the synchronous `list_messages` call, and call `schedule_refresh(cx)` after `subscribe_stream(cx)`
- [x] 1.2 `schedule_refresh`: clear `messages_loading` when the guarded transcript is applied (phase 1 ok) and when phase 1 errors
- [x] 1.3 `message_list`: render localized `messages.loading` ("Loading…") when a session is selected, messages are empty, and loading is in flight; keep `session.empty` otherwise
- [x] 1.4 App unit test: activating a session with an unreachable daemon clears messages and sets the loading flag synchronously; after the refresh settles the flag is released and the error banner state is set
- [x] 1.5 i18n: add `messages.loading` to `crates/circulo-i18n/locales/en.json`; catalog test asserts it resolves to a non-empty, non-key value

## 2. One async new-session path

- [x] 2.1 Sidebar "New session" action row: replace the inline blocking `create_session()` + `select_session` + `refresh()` with `create_new_session(window, cx)`
- [x] 2.2 Palette `PaletteItemKind::NewSession`: same replacement, then close the palette (as today)
- [x] 2.3 Verification: no direct `client.create_session()` call remains in the sidebar/palette listeners (grep); the async Home path is the only creation entry point

## 3. Migrate remaining `refresh()` call sites and delete it

- [x] 3.1 Replace `refresh()` with `schedule_refresh(cx)` in: `confirm_provider_toggle`, `archive_project`, `restore_project`, `confirm_delete_project`, `commit_rename_project`, `patch_session_project`, `set_session_agent`
- [x] 3.2 Delete the synchronous `refresh()` method; `cargo check` proves no call sites remain

## 4. Session delete confirmation

- [x] 4.1 Add `SessionOverlay::DeleteConfirm { session_id }`; `execute_session_menu_selection` index 1 calls `request_delete_session(session_id, cx)` which stages the confirm overlay
- [x] 4.2 Add `confirm_delete_session` (the current `delete_session` body: optimistic removal + background daemon delete + `schedule_refresh` on both outcomes) and rename staging to `request_delete_session`/`cancel`; render the confirm overlay in `session_overlay` (occluder + centered card, Escape/outside click cancel, `session.delete_confirm` copy, Cancel + Delete buttons)
- [x] 4.3 App unit test: request stages the confirm overlay; confirm removes the session from the list immediately; cancel leaves the session and closes the overlay

## 5. Sidebar error banner above the list

- [x] 5.1 `sidebar_body`: render the error line as a banner above the Today/Earlier sections instead of replacing them
- [x] 5.2 Verify by code review that all existing `error` setters (daemon down, send failure, rename failure, stream dropped, catalog failure, delete failure) now keep the session list visible

## 6. Localized rename rejection

- [x] 6.1 Add `settings.projects.rename_empty` ("Project name cannot be empty.") to `en.json`
- [x] 6.2 `commit_rename_project`: replace the hardcoded literal with `catalog.get("settings.projects.rename_empty")`

## 7. Verification

- [x] 7.1 `cargo test -p circulo-app -p circulo-i18n -p circulo-daemon -p circulo-persist -p circulo-core -p circulo-protocol`
- [ ] 7.2 Manual pass (docs/FLOWS.md) on macOS: daemon down shows banner + list; session switching never freezes; delete shows confirm, cancellable, cascading on confirm — pending: macOS-only app, not runnable on this Linux sandbox; automated tests cover the state transitions
