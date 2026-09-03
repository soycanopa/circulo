# Design: ui-papercut-sweep

## Context

See proposal.md — Why. Relevant current state:

- `AppShell::activate_session` (crates/circulo-app/src/shell.rs) calls `self.client.list_messages(id).unwrap_or_default()` synchronously on the UI thread inside a click handler (`select_session`), swallowing load failures.
- The Sidebar "New session" action row and `execute_palette_selection`'s `PaletteItemKind::NewSession` duplicate a blocking inline `client.create_session()` + `select_session` + `refresh()`; the Home card path (`create_new_session`) is already async and is the canonical implementation.
- `refresh()` (also shell.rs) runs `ensure_daemon` + `list_sessions` + `list_projects` + `list_messages` + `list_agents` synchronously; `ensure_daemon` may spawn and health-poll a sibling daemon (up to ~3.5 s of sleeps) and, in dev builds, rebuild it via `cargo build`.
- `sidebar_body` renders `state.error` in place of the session sections whenever `error` is set.
- The session context menu maps index 1 directly to `delete_session`, which performs the daemon delete immediately after optimistic removal.
- `commit_rename_project` uses a hardcoded English literal for the empty-name rejection.
- `schedule_refresh` already loads sessions+projects+messages in a background executor and applies the transcript guarded by `should_apply_refresh_transcript(selected == snapshot, snapshot_gen, stream_gen)`, then subscribes the stream and syncs the composer — the exact machinery session activation needs.

## Goals / Non-Goals

**Goals:**

- No UI-thread network I/O in session activation or any new-session entry point.
- A failed session load is visible (error banner), never a silent empty transcript.
- Deleting a session requires explicit confirmation; the delete request path and its optimistic-removal semantics are unchanged.
- Transient errors no longer hide the session list.
- All new user-visible copy lives in the locale catalog.

**Non-Goals:**

- Async `DaemonClient` or a connection pool.
- Toast/notification redesign of the error surface.
- Changing the daemon (`DELETE /v1/sessions/{id}` stays local-first best-effort), the protocol, or optimistic-delete behavior.
- Re-verifying or reworking scroll/stream behavior beyond what already exists.

## Decisions

### D1: Session activation loads through `schedule_refresh`

`select_session` keeps its synchronous local state work (selection, composer focus, overlay close, catalog fetch trigger, stream subscription) but the transcript fetch moves out of `activate_session`. Concretely `activate_session` clears `messages`, sets `messages_loading = true`, keeps everything else, and calls `schedule_refresh(cx)` after `subscribe_stream(cx)` (the post-subscribe call makes `schedule_refresh`'s generation snapshot match, so the fetched transcript is applied — the same ordering `create_new_session` already uses).

`schedule_refresh` clears `messages_loading` in both outcomes: when the guarded transcript is applied (phase 1 ok) and when the load fails (phase 1 error), where the existing error banner shows.

Why not a new bespoke background task: `schedule_refresh` already refreshes sessions+projects+messages with the correct guards and the error path, and the in-progress change already standardized create/delete on it. A second loader would duplicate guard logic for no benefit.

### D2: Rendering the loading state

`message_list` shows the localized `messages.loading` label ("Loading…") when a session is selected, `messages` is empty, and `messages_loading` is true; otherwise the existing empty state (`session.empty`) is used. This prevents the misleading "Write to get started" flash while a transcript is in flight.

### D3: One async new-session path

The Sidebar action row and the palette command call `create_new_session(window, cx)`; the inline blocking blocks are deleted. `create_new_session` already does `ensure_daemon` + `create_session` on the background executor, selects the session, and calls `schedule_refresh`.

### D4: `refresh()` removal

Every remaining `refresh()` call site (provider toggle confirm, project archive/restore/delete/rename, `patch_session_project`, dead `set_session_agent`) becomes `schedule_refresh(cx)`. The synchronous `refresh()` implementation is deleted so a blocking call site cannot be reintroduced by copy-paste. Note the semantic difference: `refresh()` also reloaded `available_agents`; `schedule_refresh` does not. Agents are best-effort metadata loaded on demand elsewhere (`visible_agents` composes from `available_agents`; the composer falls back to a single-entry list), and `refresh()`'s own comment called its failure non-fatal — acceptable.

### D5: Delete confirmation as a staged overlay

`execute_session_menu_selection` index 1 calls the new `request_delete_session(session_id, cx)`, which replaces the overlay with `SessionOverlay::DeleteConfirm { session_id }`. The confirm overlay mirrors the existing rename overlay (occluder + centered card, Escape and outside-click cancel, localized `session.delete_confirm` text, Cancel/Delete buttons). The current `delete_session` body (optimistic removal + background daemon delete + `schedule_refresh` on both outcomes) becomes `confirm_delete_session` and runs only when the user confirms. `activate_session` already clears overlays, so switching sessions cancels a pending confirmation.

### D6: Error banner above the list

`sidebar_body` renders the error line as the first child of the scroll content and then always renders the session sections underneath it (the "banner-above-list" layout). The `sidebar_session_sections` empty state ("No sessions yet") remains visible below the banner, which keeps the daemon-down first-run honest per `sessions-sidebar` "Daemon down is honest".

### D7: Localized rename rejection

New key `settings.projects.rename_empty` = "Project name cannot be empty."; `commit_rename_project` reads it from the catalog. Key naming follows the existing `settings.projects.*` group.

## Spec deltas

- `app-shell`: session activation and session creation must not block the UI thread on network I/O; sidebar error banner must not hide the session list; empty project rename rejected with localized copy (existing scenario "Empty rename name is rejected" already requires a human message — now sourced from the catalog).
- `sessions-sidebar`: session delete is staged — the delete request is sent only after the user confirms.

## Testing

- Shell unit tests (gpui test harness, `shell_with_dead_daemon` pattern):
  - activation with an unreachable daemon sets `messages_loading` and clears messages synchronously; after the refresh error path settles, the flag is released and `error` is set.
  - `request_delete_session` stages `SessionOverlay::DeleteConfirm`; `confirm_delete_session` optimistically removes the session and clears the selection; cancel clears the overlay without touching sessions.
- i18n catalog tests: new keys resolve to non-empty, non-key values.
- `cargo test -p circulo-app -p circulo-i18n -p circulo-daemon -p circulo-persist`.
- Manual pass (docs/FLOWS.md): boot with daemon down → sidebar shows the banner *and* the session list; open the app and switch sessions with the daemon cold → no UI freeze, transcript appears once loaded; delete from the context menu → confirm dialog appears, Cancel leaves the session, confirm removes the card immediately.
