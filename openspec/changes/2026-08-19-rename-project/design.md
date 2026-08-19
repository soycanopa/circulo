# Design: rename-project

## D1 — Backend untouched

`PATCH /v1/projects/{id}` with `PatchProjectRequest { name, description, color, folder_path }` is already implemented:

- Handler: `crates/circulo-daemon/src/http.rs:223` (`patch_project`).
- Request body: `crates/circulo-protocol/src/lib.rs:185` (`PatchProjectRequest`).
- Persistence: `crates/circulo-persist/src/store.rs:171` (`Store::update_project`).

This change does not modify the daemon, protocol, or persist crates.

## D2 — Client method

Add to `circulo_app::DaemonClient` in `crates/circulo-app/src/client.rs`:

```rust
pub fn rename_project(&self, project_id: Uuid, name: String) -> Result<Project, String> {
    self.patch(
        &format!("/v1/projects/{project_id}"),
        &PatchProjectRequest {
            name: Some(name),
            description: None,
            color: None,
            folder_path: None,
        },
    )
}
```

Reuses the existing `self.patch` helper. Returns the updated `Project` so the caller can use the server-assigned `updated_at` if needed.

## D3 — AppShell state

In `crates/circulo-app/src/shell.rs`:

- New field on `AppShell`: `pending_rename_project: Option<Uuid>` (parallel to existing `pending_delete_project`).
- New handlers:
  - `request_rename_project(id, cx)` — sets `pending_rename_project = Some(id)`; clears `pending_delete_project` to keep only one inline action per row.
  - `commit_rename_project(name, cx)` — spawns async `client.rename_project(id, name)`. On success, refreshes `projects` and `archived_projects`; clears pending. On error, surface a toast/inline copy via existing pattern (kept simple — silent in-app error state for now).
  - `cancel_rename_project(cx)` — clears `pending_rename_project`.

- Sidebar refresh after rename: the `projects` list update is enough to drive folder-label changes on session cards via the existing `session_project_label` mapper. No extra plumbing.

## D4 — Inline rename UI

In `crates/circulo-app/src/settings/projects.rs`:

Each `Project` row in `active_projects_panel` gets a **Rename** button between the row content and the existing Archive/Delete buttons (left-to-right: Rename, Archive, Delete).

When `pending_rename_project == Some(row_id)`, the row expands inline (same border-t-1 + spacing pattern as delete confirm) with:

- `crate::ui::TextInput` prefilled with `project.name`.
- **Save** button (calls `commit_rename_project(input_value, cx)`).
- **Cancel** button (calls `cancel_rename_project(cx)`).

TextInput state: managed by the existing `TextInput` machinery in `circulo-app/src/ui/input.rs`. The settings panel will hold an `Entity<TextInput>` per expand (or a single shared one keyed by id). Decision in D7 below.

## D5 — i18n

Add four keys to `crates/circulo-i18n/locales/en.json`:

```json
"settings.projects.rename": "Rename",
"settings.projects.rename_save": "Save",
"settings.projects.rename_cancel": "Cancel",
"settings.projects.rename_placeholder": "Project name"
```

No new locale file needed; default `en` is the only supported locale.

## D6 — Spec delta

In `openspec/specs/app-shell/spec.md`, add a new requirement under the Settings section (after the existing "Active projects can be archived or deleted from Settings"):

> ### Requirement: Active projects can be renamed from Settings
>
> The Projects Settings section MUST allow renaming an active project. Renaming MUST call `PATCH /v1/projects/{id}` with the new name. After a successful rename, the project list and the Sidebar folder label MUST reflect the new name.
>
> #### Scenario: Rename updates project and sidebar label
>
> - **GIVEN** an active project with sessions
> - **WHEN** the user renames it from Settings → Projects
> - **THEN** the project name updates in the panel
> - **AND** sessions belonging to that project show the new name in the Sidebar

## D7 — TextInput handling

`crate::ui::TextInput` already supports external value injection and on_change emission. Plan: hoist a single `pending_rename_input: Option<Entity<TextInput>>` on `AppShell`, created on `request_rename_project`, dropped on commit/cancel. The panel reads from it via `cx.entity()` + `cx.listener` callbacks.

If during implementation the `TextInput`'s default styling clashes with the Settings panel (it's styled for the composer), the fallback is a small `settings_text_input` helper in `settings/projects.rs` with a border + bg only. Defer that decision to the commit; not a blocker.

## D8 — Files

| File | Change |
| --- | --- |
| `crates/circulo-app/src/client.rs` | + `rename_project` method |
| `crates/circulo-app/src/shell.rs` | + `pending_rename_project` state, + 3 handlers |
| `crates/circulo-app/src/settings/projects.rs` | + Rename button, + inline expand state, + TextInput wiring |
| `crates/circulo-i18n/locales/en.json` | + 4 keys |
| `openspec/specs/app-shell/spec.md` | + new requirement |
| `openspec/changes/2026-08-19-rename-project/{proposal,design,tasks}.md` | new |

No backend, protocol, persist, or doc changes outside `app-shell` spec.

## D9 — Verification

| Step | Expected |
| --- | --- |
| `cargo check --workspace` | 0 warnings introduced by us |
| `cargo test --workspace` | 153 tests pass (no new tests; the rename path is UI-only and manual) |
| `python3 scripts/check-crate-boundaries.py` | clean |
| Manual pass | Settings → Projects → Rename → enter new name → Save → list updates; sidebar session cards show new folder label on refresh |

## D10 — Commit strategy

```
docs(openspec): add rename-project change artifacts
docs(specs): project rename in Settings
feat(i18n): add rename copy for Settings → Projects
feat(ui): inline rename affordance in projects panel
feat(app): add rename state and handlers in shell
feat(client): add rename_project HTTP method
```

Each commit is self-contained and reverts cleanly.