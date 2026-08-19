# rename-project

## Why

Once an active project exists in Circulo, the only way to change its name is to delete it and create a new one — which cascades to all its sessions (`docs/PRD.md` §7.2, PRD-PRJ-07). Non-technical users hit this immediately: typos on create, evolving project names, or simply wanting better copy after a few sessions. PRD-PRJ-03 says "El usuario puede renombrar un proyecto"; `PATCH /v1/projects/{id}` is already exposed by the daemon (`crates/circulo-daemon/src/http.rs:223`), but the app has no client method, no UI, and no copy for it.

This change closes the gap end-to-end on the app side, leaving backend untouched.

## What Changes

| Area | Outcome |
| --- | --- |
| `DaemonClient` | New `rename_project(project_id, name)` wrapping `PATCH /v1/projects/{id}` with `PatchProjectRequest { name: Some(name), description: None, color: None, folder_path: None }`. Returns the updated `Project`. |
| `AppShell` | New state `pending_rename_project: Option<Uuid>` and handlers `request_rename_project`, `commit_rename_project`, `cancel_rename_project`. On commit success, refresh `projects` and `archived_projects` (sidebar folder label updates on next session list refresh). |
| `active_projects_panel` | New **Rename** button per row. Click expands the row inline with a `TextInput` prefilled with the current name, plus **Save** and **Cancel** buttons. Same visual pattern as the existing Delete confirmation. |
| i18n | Four new keys in `en.json`: `settings.projects.rename`, `settings.projects.rename_save`, `settings.projects.rename_cancel`, `settings.projects.rename_placeholder`. |
| Spec | New requirement in `openspec/specs/app-shell/spec.md`: active projects can be renamed from Settings; rename reflects in sidebar. |

## Capabilities

### Modified Capabilities

- `app-shell`: project rename affordance in Settings.
- `circulo-protocol`: no change (PatchProjectRequest already exists).

### New Capabilities

(none — surface already exists)

## Impact

- **Crates**: `circulo-app` (client + shell + settings panel + i18n). No backend, no protocol, no persist changes.
- **External API**: none. Daemon handler already present.
- **Behavior**: a long-standing usability gap closes. No state machines, no migrations, no new endpoints.

## Non-goals

- Editing `description`, `color`, or `folder_path` (decisión cerrada: solo `name`).
- Renaming from the Sidebar (PRD-PRJ-03 is satisfied by Settings).
- Renaming archived projects (PRD-PRJ-03 is about active; archived → restore first).
- Optimistic UI with rollback (decisión cerrada: refresh simple tras ACK).
- New project PATCH fields in the protocol (none needed; existing PatchProjectRequest supports `name`).

## Open questions

(none)