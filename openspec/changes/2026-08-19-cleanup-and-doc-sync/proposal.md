# cleanup-and-doc-sync

## Why

`main` ships a working MVP slice but carries two kinds of drift:

- **Code**: `cargo check --workspace` warns on dead helpers (`next_event`, `run_turn`, `container_height`, `fn label`), unused imports across seven files, and an unused parameter. None of this breaks behavior, but it hides real surface area from future readers and slows down grep-based investigation.
- **Docs**: `README.md` still claims "Workspace scaffold only (no product behavior)" when in fact the app, daemon, and OpenCode adapter are implemented and tested. `docs/PRD.md` §12 leaves two open items (Settings final scope, sidebar ordering) that the implementation already decided. `docs/TRD.md` §15 lists nine "open" decisions, most of which the implementation has already resolved but never documented as closed. `docs/POST-MVP.md` was written when the MVP was not yet feature-complete; it is now.

This change closes the drift. No product behavior changes; no new spec deltas beyond documenting already-shipped behavior.

## What Changes

### Code cleanup

Remove warnings introduced by us (not by transitive deps). Scope is the smallest plausible set that removes the actual dead surface:

| Surface | Action |
| --- | --- |
| `circulo-adapter-opencode::Client::next_event` | Delete (only `next_event_with_activity` is used by the adapter) |
| `circulo-daemon::generate::run_turn` | Delete (only its own tests reference it; `run_assistant_turn` is the actual public surface) |
| `circulo-daemon::generate` unused imports | Remove `QuestionAnswer`, `QuestionResponse`, `PersistError` |
| `circulo-persist::store` unused imports | Remove `ComposerInteractionMode`, `ComposerPermissionMode` from the core import |
| `circulo-app::parts` unused import | Remove `BG_SIDEBAR` |
| `circulo-app::settings::projects` unused imports | Remove `prelude::FluentBuilder`, `TEXT` |
| `circulo-app::ui::menu_chip` unused import | Remove `path as icon_path` |
| `circulo-app::shell` unused import | Remove `should_apply_post_transcript` |
| `circulo-app::composer::text_layout::ComposerTextLayout::container_height` | Delete |
| `circulo-app::composer::text_layout::ComposerTextLayout::cursor_position` `line_height` parameter | Drop (prefix or remove) |
| `circulo-app::shell::label` helper | Delete |

Warnings from external crates (`block v0.1.6`, `proc-macro-error2 v2.0.1`) are out of scope.

### Document sync

| Doc | Change |
| --- | --- |
| `README.md` | Status table: replace "Workspace scaffold only (no product behavior)" with current reality. Fix Project Definition version reference (says v0.6, doc is v0.8). |
| `docs/PRD.md` §12 | Close item 3 (Settings final scope = General + Projects + Archived + Models) and item 5 (sidebar ordering = `last_message_at DESC` with NULL last). |
| `docs/TRD.md` §15 | Close all nine items: #1 HTTP localhost (no TLS in MVP), #2 bundled OpenCode spawn on :7433, #3 session binding via `agent_session_id`, #4 heroicons pipeline (`circulo-app/src/icons.rs`), #5 message list rail + cache (`content_rail` + cached markdown), #6 macOS 14 Sonoma, #7 bundle ID `app.circulo.client`, #8 i18n JSON, #9 daemon supervision via `run-app.sh`. |
| `docs/POST-MVP.md` | Reflect that the MVP is feature-complete; keep deferred changes (`opencode-attachments`, attach mode, session archive UI, rename project) as v0.2+ backlog. |
| `openspec/specs/sessions-sidebar/spec.md` | Add explicit requirement for ordering: Today and Earlier list sessions ordered by `last_message_at DESC` with NULL last, then `created_at DESC` as tie-breaker. |
| `openspec/specs/app-shell/spec.md` | Add requirement that Settings exposes General, Projects, Archived, and Models as navigation sections. |

### Out of scope

- New product behavior.
- Schema migrations.
- Dependency bumps.
- Manifest updates beyond doc/sync.
- Crate-boundary enforcement beyond `check-crate-boundaries.py`.

## Capabilities

### Modified Capabilities

- `sessions-sidebar`: ordering rule made explicit in the spec (matches the SQL already in `circulo-persist/src/store.rs`).
- `app-shell`: Settings sections enumerated (matches the implementation after `mvp-hardening`).

### New Capabilities

(none)

## Impact

- **Crates touched for cleanup only**: `circulo-adapter-opencode`, `circulo-daemon`, `circulo-persist`, `circulo-app`. No public API consumed by other crates is removed (all deletions were verified unused outside their own tests/files).
- **Docs**: `README.md`, `docs/PRD.md`, `docs/TRD.md`, `docs/POST-MVP.md`.
- **Specs**: `openspec/specs/sessions-sidebar/spec.md`, `openspec/specs/app-shell/spec.md`.
- **External API**: none.
- **Behavior**: zero. This is dead-code and doc-sync work only.

## Non-goals

- "Improving" settings scope beyond what's already implemented.
- Renaming symbols for stylistic reasons.
- Reformatting large files.
- Committing this work (commits left to the user per `AGENTS.md` §1.8).

## Open questions

(none)