# mvp-hardening

## Why

The core MVP stack (daemon, OpenCode adapter, live stream, composer, sidebar) is in `main`, but several P0 product requirements from the PRD are still backend-only: project archive/restore/delete, OpenCode health in Settings, and honest empty/error surfaces. This change closes the last planned increment in `docs/IMPLEMENTATION.md` before the MVP is feature-complete.

## What Changes

| Area | Outcome |
| --- | --- |
| **Settings → General** | Shows Circulo daemon status and OpenCode `{ available, version }` from `/v1/health`, with a retry action |
| **Settings → Projects** | Lists active projects with Archive and Delete actions; Delete requires inline confirmation |
| **Settings → Archived** | Lists archived projects with one-click Restore (no extra dialog) |
| **Session safety** | Deleting/archiving a project whose session is open clears selection and refreshes sidebar |
| **Locales** | All new copy in `en.json` |

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `app-shell`: Settings sections for health, active projects, archived projects
- `local-daemon-api`: consumed by app client (no daemon changes expected)
- `sessions-sidebar`: reflects archive/restore/delete via refreshed project/session lists

## Impact

- **Crates:** `circulo-app` (settings UI, client methods, shell state), `circulo-i18n`
- **Docs:** PRD §12 decisions closed (branding, restore confirm, auto-title, cancel stream)

## Non-goals

- Session archive UI (FLOWS §14 — out of MVP)
- Rename project from Settings (follow-up)
- Full E2E automation (manual incremental testing by user)
- QuestionCard scope decision (documented separately)

## Open questions

**Closed for this change:**

- **PRD §12.4 branding:** Sidebar keeps icon-only rail; no wordmark in MVP.
- **PRD §12.6 restore confirm:** Immediate restore on click; destructive actions (delete) keep confirmation only.
