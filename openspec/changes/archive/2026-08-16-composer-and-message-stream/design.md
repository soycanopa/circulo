## Context

See `proposal.md`. Daemon POST `/messages` already runs the fake adapter and waits. GPUI 0.2.2 still has no stock text field; the composer uses the same focused key buffer as search (Enter / Shift+Enter).

## Goals / Non-Goals

**Goals:**

- Load and display messages as plain text + a one-line tool-call summary.
- Send via daemon; lock project picker with `first_send_at`.
- Unit-test lock helper and send-disabled rules.

**Non-Goals:**

- Incremental SSE rendering.
- Markdown / diff UI.

## Decisions

### 1. Reload after POST

POST already waits for the fake turn. After it returns, GET messages.

### 2. Apply project PATCH before first send if the picker changed

If unlocked and `draft_project != session.project_id`, PATCH then send.

### 3. Enter sends, Shift+Enter newline

Recorded product convention.

### 4. Simple message body

Concatenate `MessagePart::Text`. Tool calls become `name (status)`. Task lists become their titles. Enough to see the fake turn until rich render.

## Risks / Trade-offs

- [Key-buffer composer is crude] → acceptable until a real editor; same as search.
- [Blocking send on UI thread] → run POST on the background executor like refresh.

## Migration Plan

None.

## Open Questions

None.
