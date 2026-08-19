# Tasks: rename-project

## 1. Client

- [x] 1.1 Add `DaemonClient::rename_project(project_id, name) -> Result<Project, String>` to `crates/circulo-app/src/client.rs`
- [x] 1.2 Wire `PatchProjectRequest` import in `client.rs` (if not already present)

## 2. AppShell state

- [x] 2.1 Add `pending_rename_project: Option<Uuid>` field to `AppShell` in `crates/circulo-app/src/shell.rs`
- [x] 2.2 Initialize the field in `AppShell::new` and any `Default::default` paths
- [x] 2.3 Clear the field on every other relevant transition (cancel, commit, archive, delete, restore) to avoid stale state
- [x] 2.4 Implement `request_rename_project(id, cx)` handler
- [x] 2.5 Implement `commit_rename_project(name, cx)` handler that calls `DaemonClient::rename_project` async and refreshes lists on success
- [x] 2.6 Implement `cancel_rename_project(cx)` handler

## 3. UI

- [x] 3.1 Add Rename button to each row in `active_projects_panel` (`crates/circulo-app/src/settings/projects.rs`)
- [x] 3.2 Render inline expand with `TextInput` + Save + Cancel when `pending_rename_project == Some(row.id)`
- [x] 3.3 Wire TextInput → Save button → `commit_rename_project`
- [x] 3.4 Wire Cancel button → `cancel_rename_project`
- [x] 3.5 Mirror the inline affordance visual style of the existing delete confirmation

## 4. i18n

- [x] 4.1 Add `settings.projects.rename`, `rename_save`, `rename_cancel`, `rename_placeholder` keys to `crates/circulo-i18n/locales/en.json`

## 5. Spec

- [x] 5.1 Add "Active projects can be renamed from Settings" requirement + scenario in `openspec/specs/app-shell/spec.md`

## 6. OpenSpec artifacts

- [x] 6.1 `proposal.md`, `design.md`, `tasks.md` in `openspec/changes/2026-08-19-rename-project/`

## 7. Verification

- [x] 7.1 `cargo check --workspace` clean (no new warnings)
- [x] 7.2 `cargo test --workspace` — 153 tests pass
- [x] 7.3 `python3 scripts/check-crate-boundaries.py` clean
- [x] 7.4 Manual: rename an active project, list updates, sidebar folder label updates on next refresh