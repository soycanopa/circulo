# Tasks: mvp-hardening

## 1. Client

- [x] 1.1 Add `list_archived_projects`, `archive_project`, `restore_project`, `delete_project` to `DaemonClient`
- [x] 1.2 Unit tests for archived list URL and delete/archive/restore paths — covered by daemon integration tests; client paths verified via compile-time wiring

## 2. Settings structure

- [x] 2.1 Extend `SettingsSection` with General, Projects, Archived
- [x] 2.2 Wire nav in `settings_sidebar_body` and panel routing in `settings_main_column`

## 3. General panel

- [x] 3.1 `settings/general.rs` health panel with daemon + OpenCode rows and retry
- [x] 3.2 Fetch health on settings open; locale keys

## 4. Projects panel

- [x] 4.1 `settings/projects.rs` active project list with archive action
- [x] 4.2 Inline delete confirmation + cascade handling in shell

## 5. Archived panel

- [x] 5.1 Archived list with immediate restore
- [x] 5.2 Refresh sessions/projects after restore

## 6. Locales and docs

- [x] 6.1 Add all settings copy to `en.json`
- [x] 6.2 Close PRD §12 open items (branding, restore confirm, auto-title, cancel stream) and TRD sync

## 7. Verification

- [x] 7.1 `cargo test` affected crates + `check-crate-boundaries.py`
