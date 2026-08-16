## Why

The workspace compiles, but Circulo has no shared domain types or versioned API contract. Persistence, the daemon, and the UI cannot be built against a single JSON-serializable model until those types exist.

## What Changes

- Add domain entities in `circulo-core`: Project, Session, Message, MessagePart, ToolCall, Task, Question, preferences (`sidebar.view`).
- `project_id` is optional. Unassigned sessions are the special `Sessions` folder (`project_id` null).
- Encode the lock: `project_id` may change only before the first sent user message.
- Add `circulo-protocol`: `api_version`, typed SSE events from the TRD draft, and API errors with stable codes + human messages.
- JSON serde: `snake_case`, UUIDs, RFC3339 timestamps. Roundtrip tests.
- Question remains in the model only (no UI).

## Capabilities

### New Capabilities

- `domain-model`: Circulo entities, status machines, and assignment rules (optional project, lock after first send, archive vs delete is a persist concern).
- `circulo-protocol`: Versioned app↔daemon contract types (events, errors, `api_version`).

### Modified Capabilities

- (none)

## Non-goals

- No SQLite, no HTTP/SSE server, no GPUI, no adapter trait implementation.
- No worktree / working directory fields.
- Does not implement restore/archive persistence (types only).

## Impact

- Crates: `circulo-core`, `circulo-protocol`.
- New deps: `serde`, `serde_json`, `uuid`, `time`.
- Unblocks `local-persistence` and later daemon/UI changes.

## Open questions (not resolved here)

- Exact human copy for error messages (locale keys come later; protocol carries English `message` strings as data).
