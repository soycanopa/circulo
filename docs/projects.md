# Circulo — Projects (v1.0)

- **Status:** Implemented (2026-11-26), branch `feature/projects`.
- **Related:** [PRD](prd.md) · [TRD](trd.md) · [UI](ui.md) · [Implementation](implement.md).
- **Reference:** Waku's composer project chip and session row. Ideas only; no code copied.

## Goal

A project is a folder on the daemon host, and a session is located in one. The
sidebar keeps the reference's row — title, project below, time on the right —
and the project is chosen from the composer, not from a second sidebar list.

After this change:

- The composer's project chip opens the folders the user added, `New project…`
  and `No project`.
- Choosing a folder starts or reuses a session located in it, carrying the
  draft. A started session is never moved to another folder.
- `New Task` stays in the current session's project and reuses a blank task for
  it, as the reference does.
- The project's agent runs in that folder (`cwd` per session).
- A session without a project keeps working.

## Decisions

1. **The canonical path is the key.** No UUID, no FK. Sessions already store a
   path in `sessions.project`; the `projects` table is the picker's list. Two
   rows cannot share a path. Removing a project removes the entry, not the
   folder or its sessions.
2. **The daemon owns the path.** The client opens the native picker (loopback
   is the only transport today). `AddProject` sends the path; the daemon
   canonicalizes it, requires a directory, derives the name from the last
   component, and answers `ProjectAdded { path }` with the canonical path
   before the refreshed `Projects` list. The chip then starts a session in that
   path.
3. **The sidebar does not list projects.** The session row keeps the reference
   shape: title, project with a folder icon, and the trailing time (`Working
   12s` while the open turn runs, otherwise how long ago). The project picker
   lives in the composer, where the folder belongs to the task being composed.
4. **Choosing a project navigates, it does not mutate.** It opens the blank
   session for that project if one exists, otherwise starts one. A session with
   a prompt is never repointed: its agent has a working directory. `No project`
   does the same for a projectless blank task.
5. **Per-session cwd is in `SessionRef`.** `start_turn` gets the session's
   canonical folder; OpenCode spawns that session's child there. A projectless
   session falls back to the driver's configured folder. `options`/`set_option`
   keep the driver's folder: they only discover/apply agent-global settings, and
   the stored overrides are re-applied on the next real turn.
6. **No branch in the data yet.** The footer keeps its previous shape
   (`project · Local · branch`); the branch is still the mock label until Git
   checkpoints land. Reading `.git/HEAD` belongs to the Git crate, not here.

## Schema (version 4)

```sql
CREATE TABLE IF NOT EXISTS projects (
    path        TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    created_at  INTEGER NOT NULL
);
```

The migration backfills from existing sessions:

```sql
SELECT DISTINCT project FROM sessions WHERE project IS NOT NULL AND project <> '';
```

Store: `projects()` (sorted by name), `add_project(path, name)` (upsert),
`remove_project(path)`. The folder is never touched.

## Wire (additive, version stays 3)

- `ClientRequest::ListProjects`
- `ClientRequest::AddProject { path }`
- `ClientRequest::RemoveProject { path }`
- `ServerMessage::Projects { projects: Vec<ProjectSummary> }`
- `ServerMessage::ProjectAdded { path }` — the canonical path the daemon
  actually stored, sent before `Projects`.
- `ProjectSummary { path, name }`

`StartSession { project: Option<String> }` already existed and now carries a
real canonical path.

## Core

- `projects()` and `remove_project(path)`.
- `add_project(path) -> (canonical, projects)`: canonicalize, require a
  directory, derive the name, upsert, return the list.
- `run_turn` builds `SessionRef { id, resume, cwd: session.project.as_deref() }`.
- A memory-only core has no project list and refuses to store one.

## Provider

`SessionRef` gains `cwd: Option<&str>`. OpenCode's `attach` uses the session's
folder for `Agent::spawn`, `session/new` and `session/load`; `None` falls back
to the driver's folder (the daemon's cwd / `CIRCULO_PROJECT`).

## UI

- `Shell` keeps `projects`, `current_project` (the open session's canonical
  path, updated at once when a chip choice starts), `project_open` and
  `adding_project`.
- The composer footer's project chip is the picker. Its menu lists projects
  (the current one marked), `New project…` (native picker) and `No project`.
- `New project…` sends `AddProject`; on `ProjectAdded`, the shell starts a
  session in the canonical path.
- `New Task` uses the current project and reuses a blank task for it.
- The sidebar row stays title / project / time. The open turn shows
  `Working Ns`; the rest show the relative time.
- Live messages are filtered by session, so a background turn cannot leak into
  another session's transcript.

## Verification

- Store: v3 → v4 migration backfills session projects; add/remove/list round
  trip; duplicate add is idempotent.
- Core: the project path reaches `SessionRef.cwd`; a projectless session gets
  `None`; add validates a directory and canonicalizes.
- OpenCode: a scripted agent's `session/new` receives the session's folder, and
  a projectless session gets the driver's fallback.
- Daemon/client: `AddProject` → `Projects`; `StartSession { project }` → the
  session summary carries the path; removing a project keeps its sessions.
- UI smoke: the project menu opens from the chip, choosing a project starts a
  session, the sidebar row reads title / project / time, and the agent reports
  `cwd=/private/tmp/circulo-proj-a`. No duplicate blank task when the same
  project is chosen twice.

## Out of scope

- Git worktrees per session and the real branch name (Phase 3).
- Renaming a project, moving its folder, or deleting files.
- A project-list section in the sidebar: the reference keeps it in the composer.
- Remote folder picking on another host (the wire carries a path; the picker
  moves when remote lands).
