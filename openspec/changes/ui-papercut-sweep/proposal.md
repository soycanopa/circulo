# Proposal: ui-papercut-sweep

## Why

An audit of the main flows (Arranque, Primera vez, Crear/navegar sesiones, Enviar y streamear, Sidebar, Settings, borrar/renombrar) found four high-visibility papercuts:

1. **Session activation and two of the three "New session" entries still block the UI thread.** `fix-composer-models-and-session-delete` removed the blocking `refresh()` from Home create / delete, but `activate_session` still runs a synchronous `list_messages` on the UI thread (`unwrap_or_default()`), and the Sidebar "New session" row and the palette "New session" command still run `create_session()` + the full blocking `refresh()` inline in their click handlers. `refresh()` calls `ensure_daemon`, which can spawn, rebuild (`cargo build` in dev), and health-poll the daemon for seconds — a visible freeze. The Home "New session" card already uses the async `create_new_session`; the other two surfaces were left behind.
2. **Activation failures are swallowed.** `list_messages` uses `unwrap_or_default()`: when the load fails the transcript silently renders the empty state ("Write to get started") with no error, which can mislead the user into writing into a session whose history failed to load.
3. **Deleting a session has no confirmation.** The context menu deletes on one click and the transcript is unrecoverable. The catalog already ships `session.delete_confirm` ("Delete this session? This can't be undone.") — unused copy that documents intent. Project delete already confirms; session delete should too.
4. **Any transient error hides the whole session list.** `sidebar_body` renders `state.error` *instead of* the Today/Earlier sections. A failed send, a failed rename, or a boot-time catalog failure makes the navigation surface disappear until a later success clears the error.
5. **One hardcoded UI string.** `commit_rename_project` rejects an empty name with a literal `"Project name cannot be empty."` outside the locale catalog (violates PRD-APP-06 "Toda cadena visible al usuario vive en un catálogo de locale").

## What Changes

- `activate_session` performs no network I/O on the UI thread: it clears the transcript, sets a `messages_loading` flag (rendered as "Loading…", new `messages.loading` key), and the daemon fetch happens in the background via the existing `schedule_refresh`, which already applies transcripts with generation guards and surfaces failures through the existing sidebar error banner.
- The Sidebar "New session" row and the palette "New session" command route through the existing async `create_new_session` path instead of duplicating a blocking inline create+refresh.
- Every remaining synchronous `refresh()` call site (provider toggle, project archive/restore/delete/rename, session project patch) switches to `schedule_refresh`; the blocking `refresh()` method is deleted.
- Session delete gains a confirmation step: choosing Delete in the session context menu shows a confirm overlay with the existing `session.delete_confirm` copy; the daemon call only fires on explicit confirm.
- The sidebar error renders as a banner above the Today/Earlier sections instead of replacing them.
- New locale key `settings.projects.rename_empty` ("Project name cannot be empty.") replaces the hardcoded literal.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `app-shell`: new requirement — session activation and New-session creation must not block the UI thread on network I/O; requirement — the sidebar error banner must not hide the session list; requirement — empty project renames are rejected with localized copy.
- `sessions-sidebar`: new requirement — deleting a session requires an explicit confirmation before the delete request is sent.

## Impact

- `crates/circulo-app/src/shell.rs` — `activate_session`, sidebar/palette new-session handlers, `refresh()` call-site migration, sidebar error banner, locale lookup for rename rejection, `messages_loading` state.
- `crates/circulo-app/src/session_overlay.rs` — `SessionOverlay::DeleteConfirm` variant and overlay rendering.
- `crates/circulo-app/src/stream.rs` — no changes (guards reused as-is).
- `crates/circulo-i18n/locales/en.json` — two new keys (`messages.loading`, `settings.projects.rename_empty`).
- Tests: shell unit tests for activation loading state, delete-confirm staging, and new-session path parity; existing test suites must stay green.

## Non-goals

- No protocol/daemon changes: `POST /v1/sessions`, `DELETE /v1/sessions/{id}`, and the SSE contract keep their contracts; delete stays optimistic + local-first as specified by `fix-composer-models-and-session-delete`.
- No general async refactor of `DaemonClient` (it remains a synchronous HTTP client used only from background executors).
- No rework of the error model (toasts, banners in the main column) — only the sidebar list-visibility behavior is corrected.
- No new navigation, keyboard, or a11y features beyond what the touched flows already have.

## Open questions

(none — all decisions are closed by existing specs/decisions, only implementation details change)
