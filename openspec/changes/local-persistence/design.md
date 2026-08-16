## Context

See `proposal.md`. Types live in `circulo-core`. This change only fills `circulo-persist`. Path decision from TRD: Application Support, confirmed here as `Circulo/circulo.sqlite`.

## Goals / Non-Goals

**Goals:**

- Open or create a SQLite file, run migrations, CRUD the domain graph.
- Enforce FK cascade and the assignment lock.
- Test against temp databases, not the real Application Support path.

**Non-Goals:**

- HTTP, UI, adapters.
- Encrypting the DB.

## Decisions

### 1. `rusqlite` with `bundled`

**Why:** no system SQLite dependency. Fine for a desktop daemon.

**Alternative:** sqlx async. Rejected — persist is sync and called from the daemon; keep the crate small.

### 2. Hand-rolled `schema_migrations` table

**Why:** one SQL file / numbered migration is enough. Avoid another crate until we feel pain.

### 3. `Store::open(path)` + `Store::open_default()`

Default: `~/Library/Application Support/Circulo/circulo.sqlite` via the `directories` crate (`ProjectDirs` qualifier `app`, org `Circulo`, app `Circulo`). Tests never call `open_default`.

### 4. Message parts as JSON blobs

`message_parts.payload` is the serde JSON of `MessagePart`. Streaming updates replace the payload at `(message_id, position)`.

**Alternative:** fully normalized tool_call columns. Rejected for now — the part union would fight the schema.

### 5. Foreign keys and WAL

`PRAGMA foreign_keys = ON` on every connection. `journal_mode = WAL`.

`sessions.project_id` → `projects(id)` ON DELETE CASCADE.  
`messages.session_id` → `sessions(id)` ON DELETE CASCADE.  
`message_parts.message_id` → `messages(id)` ON DELETE CASCADE.

### 6. Visibility helper

Main listings: session `status = 'active'` AND (`project_id` IS NULL OR project `status = 'active'`).

Archived project list: `projects.status = 'archived'`.

### 7. First send

Saving a `MessageRole::User` message on a session with `first_send_at` null sets `first_send_at` (and `last_message_at`) in the same transaction.

### 8. List order

`last_message_at DESC NULLS LAST`, then `created_at DESC`. Recorded here; product did not lock this.

## Risks / Trade-offs

- [JSON parts harder to query] → acceptable; chat load is by session.
- [Application Support path wrong for later branding] → one constant; easy to change.
- [rusqlite 0.40 API] → pin in workspace; tests cover the store.

## Migration Plan

Fresh DBs only. No user data exists yet.

## Open Questions

None that block this change.
