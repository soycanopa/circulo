## Context

See `proposal.md`. Daemon API exists. App shell exists. GPUI 0.2.2 has no stock text input; search is a focused query buffer plus client-side title filter.

## Goals / Non-Goals

**Goals:**

- Load sessions/projects over HTTP.
- View switcher writes `/v1/preferences`.
- New session / New project.
- Relative time helper with tests.
- One sibling-binary spawn if health fails.

**Non-Goals:**

- Composer, message list from daemon, settings.
- Robust process supervisor.

## Decisions

### 1. `ureq` blocking client on GPUI background tasks

**Why:** app must not take tokio just for a few REST calls. `ureq` is small. Calls run on `background_executor`.

### 2. Join lists in the UI

`GET /v1/sessions` + `GET /v1/projects`, group in memory for Groups view.

### 3. Search is client-side title contains

Avoid a custom editor. Clicking Search focuses a query; keys append; filter `title`.

### 4. Relative time from `last_message_at` else `created_at`

Matches UX spec.

### 5. Single spawn of `circulo-daemon` next to `circulo-app`

If `GET /v1/health` fails. No restart loop.

## Risks / Trade-offs

- [Search UX is crude] → good enough until a real input lands with composer.
- [Spawn may fail under `cargo run` if cwd differs] → look at `current_exe()` parent (`target/debug`).

## Migration Plan

None.

## Open Questions

Supervisor policy beyond one spawn.
