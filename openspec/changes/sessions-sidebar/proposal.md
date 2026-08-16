## Why

The window shell is a placeholder. Users cannot see, create, or switch sessions, or change Sessions/Groups view. That is the first usable product surface after chrome.

## What Changes

- Talk to the local daemon (`127.0.0.1:7432`) for projects, sessions, and the saved sidebar view.
- Sessions view: flat list with name, relative time, project or “No project”.
- Groups view: active projects with nested sessions; empty state CTA **New project**.
- New session creates an unassigned session and selects it.
- Remember view via `/v1/preferences`; if missing or unloadable, default to Sessions.
- Search filters the Sessions list (title).
- If the daemon is down, show an English error from the locale catalog. Try to start a sibling `circulo-daemon` once.

## Capabilities

### New Capabilities

- `sessions-sidebar`: Sidebar session/project lists, view switcher, new session/project, search, daemon error.

### Modified Capabilities

- (none)

## Non-goals

- No composer send, no message stream, no settings panel.
- No project picker in the composer (next change).
- No Heroicons.

## Impact

- `circulo-app` gains a small HTTP client (ureq) and depends on `circulo-core` + `circulo-protocol`.
- Still must not depend on persist or OpenCode.

## Open questions (not resolved here)

- Full daemon supervisor (restart loops, logs). This change only does a single sibling spawn attempt.
