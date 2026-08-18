# Design: mvp-hardening

## D1 — Settings layout

Extend `SettingsSection` with three nav items: **General**, **Projects**, **Archived**. The main column switches panels by `settings_section`. Models stays as a fourth section (already shipped).

```
Settings sidebar nav
  General    → health panel
  Projects   → active project list + archive/delete
  Archived   → archived list + restore
  Models     → existing models panel
```

## D2 — Client API

Add to `DaemonClient`:

- `list_archived_projects()` → `GET /v1/projects?status=archived`
- `archive_project(id)` → `POST /v1/projects/{id}/archive`
- `restore_project(id)` → `POST /v1/projects/{id}/restore`
- `delete_project(id)` → `DELETE /v1/projects/{id}`

`health()` already returns `HealthResponse` with optional `opencode`.

## D3 — Health panel

On open Settings → General, call `health()` (background). Show:

- Daemon: ok / error from `daemon` field
- OpenCode: available + version, or unavailable + `adapter_message` mapped to locale when possible
- **Retry** button re-fetches health

Store last health in `AppShell` as `Option<HealthResponse>` + `health_error: Option<String>`.

## D4 — Projects panel

Load `list_projects()` on entering Projects section (or on settings open). Each row: name, optional folder path truncated, actions **Archive** and **Delete**.

**Delete flow:** first click sets `pending_delete_project: Option<Uuid>`. Row expands inline confirm strip with message including session count if cheap to fetch, else generic copy. Confirm calls `delete_project`; Cancel clears pending.

**Archive flow:** immediate `archive_project` + refresh lists. If selected session belonged to archived project, deselect.

## D5 — Archived panel

Load `list_archived_projects()`. Each row: name + **Restore** (immediate, no dialog per product decision). After restore, refresh active projects and sessions.

## D6 — Refresh strategy

After archive/restore/delete, call existing `refresh()` on shell to reload sessions and projects. Clear `selected` if its `project_id` no longer visible or session was cascade-deleted.

## D7 — Files

| File | Change |
| --- | --- |
| `settings/mod.rs` | New sections enum |
| `settings/general.rs` | Health panel (new) |
| `settings/projects.rs` | Active + archived panels (new) |
| `client.rs` | Project lifecycle HTTP |
| `shell.rs` | State, handlers, settings_main_column routing |
| `locales/en.json` | New keys |
