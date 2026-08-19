# Design: cleanup-and-doc-sync

## D1 — Scope guardrails

The change touches two surfaces: Rust crates (cleanup) and Markdown (sync). Both are non-feature. The change must not alter:

- The public HTTP/SSE surface of `circulo-daemon`.
- The OpenCode adapter's event translation.
- The SQLite schema or migrations.
- Any user-visible UI behavior, copy in locales, or settings nav order.

If a refactor seems desirable while editing (e.g. "this function name is awkward"), defer it. Cleanup is removal only.

## D2 — Dead-code removal rules

For each candidate deletion, the rule is:

1. Grep the workspace for the symbol.
2. The only acceptable matches are: definition site, the symbol's own tests, or comments/docs that mention it.
3. If a match exists in any other crate, in `circulo-app::shell`, or in production paths of `circulo-daemon` / `circulo-adapter-opencode`, **do not delete** — it's load-bearing, the warning is misleading, and we need to investigate.

Verified scope before writing this design:

| Symbol | Matches outside own file? | Verdict |
| --- | --- | --- |
| `circulo_adapter_opencode::client::Client::next_event` | None in production; `next_event_with_activity` is used in `lib.rs:204`. | Delete. |
| `circulo_daemon::generate::run_turn` | None outside `generate.rs` tests. `run_assistant_turn` (different symbol) is what `http.rs` uses. | Delete. |
| `circulo_app::composer::text_layout::ComposerTextLayout::container_height` | None. | Delete. |
| `circulo_app::shell::label` (helper at `shell.rs:3558`) | None. | Delete. |
| `ComposerTextLayout::cursor_position`'s `line_height` parameter | Parameter is unused inside the function. | Drop the parameter (call site has it; update call site too). |

Imports to remove were verified by grep per file.

## D3 — Document sync targets

| Doc | Source of truth we sync to |
| --- | --- |
| `README.md` Status table | Branch `main` today: app, daemon, adapter implemented; 128 tests pass. |
| `README.md` Project Definition version | Header of `Circulo-Project-Definition.md` says `Versión 0.8`. |
| `docs/PRD.md` §12 item 3 | `circulo-app/src/settings/mod.rs` defines `SettingsSection = General, Projects, Archived, Models`. |
| `docs/PRD.md` §12 item 5 | `circulo-persist/src/store.rs` queries: `ORDER BY last_message_at IS NULL, last_message_at DESC, created_at DESC`. |
| `docs/TRD.md` §15 #1 | `circulo-daemon` listens on plain HTTP; `axum` is built without TLS. |
| `docs/TRD.md` §15 #2 | `circulo-daemon` spawns `opencode serve` on port 7433 via bundled `.app`. |
| `docs/TRD.md` §15 #3 | `circulo-persist` persists `agent_session_id` per session; adapter uses it. |
| `docs/TRD.md` §15 #4 | `circulo-app/src/icons.rs` resolves heroicons paths used across the UI. |
| `docs/TRD.md` §15 #5 | `circulo-app/src/ui/layout.rs` (`content_rail`) + `circulo-app/src/parts.rs` markdown cache. |
| `docs/TRD.md` §15 #6 | User-confirmed: macOS 14 Sonoma. |
| `docs/TRD.md` §15 #7 | User-confirmed: bundle ID `app.circulo.client`. |
| `docs/TRD.md` §15 #8 | `crates/circulo-i18n/locales/en.json`. |
| `docs/TRD.md` §15 #9 | `scripts/run-app.sh` spawns the daemon. |
| `docs/POST-MVP.md` | `mvp-hardening` change is archived, MVP feature-complete per POST-MVP §5. |

## D4 — Spec sync targets

Two spec files get additions. Both are *additions of an explicit requirement* for behavior that already ships. No existing requirement is modified.

### `openspec/specs/sessions-sidebar/spec.md`

New requirement:

> ### Requirement: Today and Earlier list sessions in `last_message_at` desc order
>
> Within Today and within Earlier, sessions MUST be ordered by `last_message_at DESC`, with `NULL` values placed last. When two sessions share the same `last_message_at`, `created_at DESC` breaks the tie. The implementation MAY push the ordering down to SQL (`ORDER BY last_message_at IS NULL, last_message_at DESC, created_at DESC`).
>
> Scenario: New session ranks above untouched sessions
> - **GIVEN** a fresh session with no messages and an existing session with a message today
> - **WHEN** Today renders
> - **THEN** the existing session appears before the new one (real activity above no-activity)

### `openspec/specs/app-shell/spec.md`

New requirement:

> ### Requirement: Settings exposes General, Projects, Archived, and Models
>
> The Settings surface MUST expose exactly four navigation sections in this order: **General**, **Projects**, **Archived**, **Models**. Removing or renaming a section requires a new OpenSpec change.
>
> Scenario: All four sections are visible
> - **GIVEN** the user opens Settings
> - **WHEN** the sidebar nav renders
> - **THEN** General, Projects, Archived, and Models are listed in that order
> - **AND** each section routes to its dedicated panel

## D5 — Files touched

### Code

| File | Change |
| --- | --- |
| `crates/circulo-adapter-opencode/src/client.rs` | Delete `next_event`. |
| `crates/circulo-daemon/src/generate.rs` | Delete `run_turn`; trim unused imports. |
| `crates/circulo-persist/src/store.rs` | Trim unused imports. |
| `crates/circulo-app/src/parts.rs` | Trim unused import. |
| `crates/circulo-app/src/settings/projects.rs` | Trim unused imports. |
| `crates/circulo-app/src/ui/menu_chip.rs` | Trim unused import. |
| `crates/circulo-app/src/shell.rs` | Trim unused import; delete `fn label`. |
| `crates/circulo-app/src/composer/text_layout.rs` | Delete `container_height`; remove unused `line_height` param. |

### Docs

| File | Change |
| --- | --- |
| `README.md` | Status table + Project Definition version. |
| `docs/PRD.md` | §12 items 3 and 5. |
| `docs/TRD.md` | §15 items 1–9. |
| `docs/POST-MVP.md` | §1 / §5 wording; backlog remains. |

### Specs

| File | Change |
| --- | --- |
| `openspec/specs/sessions-sidebar/spec.md` | New ordering requirement. |
| `openspec/specs/app-shell/spec.md` | New Settings-sections requirement. |

## D6 — Verification

| Step | Expected |
| --- | --- |
| `cargo check --workspace` | No new warnings (existing deps warnings remain). |
| `cargo test --workspace` | 128 tests pass. |
| `python3 scripts/check-crate-boundaries.py` | Clean. |
| `rg -n 'TODO\|FIXME\|todo!\|unimplemented!' crates/` | Empty. |

## D7 — Commit strategy

No commits in this agent pass. The user commits per AGENTS.md §1.8 after reviewing the diff. Suggested commit slicing if the user asks for help later:

1. `chore(adapter-opencode): remove dead Client::next_event`
2. `chore(daemon): remove dead generate::run_turn and trim imports`
3. `chore(persist): trim unused core imports`
4. `chore(app): trim unused imports in parts, settings, shell, menu_chip`
5. `chore(app): remove dead text_layout::container_height and shell::label`
6. `docs(readme): sync Status table and Project Definition version`
7. `docs(prd): close §12 items 3 and 5`
8. `docs(trd): close §15 items 1–9`
9. `docs(post-mvp): note MVP is feature-complete`
10. `docs(specs): explicit sidebar ordering`
11. `docs(specs): enumerate Settings sections`