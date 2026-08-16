## Why

Domain types exist only in memory. Circulo cannot keep projects, sessions, or messages across process restarts, and cannot enforce cascade delete or archive visibility until there is a SQLite store.

## What Changes

- Implement `circulo-persist` on SQLite with versioned migrations.
- Persist projects, sessions (`project_id` nullable, `ON DELETE CASCADE`), messages and parts, and `sidebar.view`.
- Default DB path: `~/Library/Application Support/Circulo/circulo.sqlite`. Tests use a temp file.
- Archive vs delete: archive updates status; delete removes the project and its sessions/messages.
- Restore sets a project back to `active`.
- Main list queries hide archived projects and their sessions.
- Unassigned sessions are `project_id IS NULL`.
- Title search for sessions.
- Honor `Session::assign_project` (lock after first send).

## Capabilities

### New Capabilities

- `local-persistence`: Local SQLite store for Circulo entities and the sidebar view preference.

### Modified Capabilities

- (none)

## Non-goals

- No HTTP daemon, no GPUI, no OpenCode.
- No cloud sync.
- No full-text search beyond session title.
- Does not choose TLS or spawn OpenCode.

## Impact

- Crate `circulo-persist`.
- New deps: `rusqlite` (bundled), `directories`, `tempfile` (dev).
- Unblocks `local-daemon-api`.

## Open questions (not resolved here)

- Exact Application Support bundle name beyond `Circulo` (closed here as `Circulo` unless product later changes the app name).
