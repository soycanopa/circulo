## Context

See `proposal.md`. Workspace crates exist and are empty. Types follow `docs/TRD.md` §5–6 and the project definition model. No HTTP or SQLite in this change.

## Goals / Non-Goals

**Goals:**

- `circulo-core` owns entities and assignment rules.
- `circulo-protocol` owns `api_version`, SSE event enum, `ApiError`.
- Serde JSON `snake_case` with tagged unions for parts/outputs/events.
- Unit tests for roundtrip and the project-lock rule.

**Non-Goals:**

- Persistence, HTTP, GPUI, adapters.
- Locale catalogs (error `message` is English data for now).
- Implementing archive/delete side effects.

## Decisions

### 1. `uuid` + `time` + `serde` / `serde_json`

**Choice:** `uuid` (serde feature), `time` (serde + parsing), workspace-wide.

**Why:** UUIDs are required. RFC3339 via `time::OffsetDateTime` avoids chrono’s heavier surface.

**Alternative:** stringly IDs and timestamps. Rejected — too easy to drift.

### 2. Internally tagged enums

**Choice:** `#[serde(tag = "type", rename_all = "snake_case")]` on `MessagePart`, `ToolOutput`, `ProtocolEvent`. Status enums: `rename_all = "snake_case"` as strings.

**Why:** matches the JSON example in the project definition (`"type": "text"`).

### 3. Project lock is a pure function

**Choice:** `Session::can_change_project(has_sent_user_message: bool) -> bool` and `Session::assign_project(...) -> Result<..., DomainError>`.

**Why:** persist/HTTP will call this; no hidden global state.

**Alternative:** infer “first send” from `messages.len()`. Not possible in core without loading messages. Caller passes the boolean (or we add `has_sent_user_message` on Session later). For this change, the function takes the fact as input. Also store `first_send_at: Option<OffsetDateTime>` on Session so the rule can be applied without the message list.

**Choice:** add `first_send_at: Option<OffsetDateTime>` on Session. Set when the first user message is sent (later changes). `can_change_project` is `first_send_at.is_none()`.

### 4. `api_version` is `u32`, current `1`

Constant `circulo_protocol::API_VERSION`. Every `ProtocolEvent` and `ApiError` includes it.

### 5. Error codes as enum, serialized snake_case

Examples: `project_assignment_locked`, `not_found`, `invalid_request`. Message is a static English sentence for now.

### 6. Protocol events wrap core types by id + payload

Do not invent extra events. Payloads:

- `server.connected` — no session
- `session.message.created` / `updated` / `completed` / `failed` — session_id + message
- `session.part.appended` / `updated` — session_id, message_id, part
- `session.tool_call.updated` — session_id, message_id, tool_call

### 7. Question stays in core

No protocol-only omission. UI later ignores or fallback-renders.

## Risks / Trade-offs

- [Event shapes change when HTTP lands] → version bump or an explicit later change; tests lock current JSON.
- [English error strings vs i18n] → daemon can map `code` to locale later; keep `message` as fallback.
- [`first_send_at` not in original struct sketch] → needed for the lock without loading messages. Documented here.

## Migration Plan

None. Types only.

## Open Questions

None for this change.
