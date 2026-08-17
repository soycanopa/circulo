## 1. Time helpers

- [x] 1.1 Add `is_same_local_day` and tests in `timefmt.rs`
- [x] 1.2 Add `partition_sessions_by_day` in `client.rs` with tests

## 2. Sidebar UI

- [x] 2.1 Remove ViewSwitcher, Groups branch, and `view` state from `shell.rs`
- [x] 2.2 Render Today and Earlier sections with `session_row` layout
- [x] 2.3 Omit empty sections; empty state when no sessions match

## 3. Locales and composer

- [x] 3.1 Update `en.json`: Today, Earlier, Without Folder; remove Groups keys
- [x] 3.2 Use `session.without_folder` in composer picker

## 4. Remove sidebar_view stack

- [x] 4.1 Remove `SidebarView` from core, protocol, persist, daemon
- [x] 4.2 Remove client `set_view` / preferences usage in app
- [x] 4.3 Update or remove related tests

## 5. Docs

- [x] 5.1 Update PRD, UX-UI, FLOWS, TRD, README, AGENTS.md

## 6. Verification

- [x] 6.1 Run `cargo test` for affected crates
- [x] 6.2 Manual check: Today/Earlier partition, search, card layout
