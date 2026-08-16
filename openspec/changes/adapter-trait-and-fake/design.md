## Context

See `proposal.md`. TRD §7: the trait translates Circulo ↔ provider. Fake is for UI and daemon tests.

## Goals / Non-Goals

**Goals:**

- A small `AgentAdapter` trait in `circulo-adapter`.
- A deterministic `FakeAdapter` in `circulo-adapter-fake`.
- Events reuse core `ToolCall` / `Task` types so the daemon can map them to protocol events later.

**Non-Goals:**

- Async runtime.
- OpenCode client.
- Wiring into the daemon binary.

## Decisions

### 1. Sync emit callback

```rust
fn generate(&self, request: GenerateRequest, emit: &mut dyn FnMut(AdapterEvent)) -> Result<(), AdapterError>
```

**Why:** no tokio yet; the real adapter can still emit incrementally. Tests collect into a `Vec`.

**Alternative:** `Iterator` / channel. Callback is enough for the fake and for the first daemon.

### 2. Event set

`TextDelta`, `TaskList`, `ToolCallStarted`, `ToolCallUpdated`, `Completed`, `Failed`.

No extra events.

### 3. Fake script

Always the same, independent of prompt text (prompt is stored on the request for later adapters):

1. Text: `"Working on it."`
2. Task list: two tasks (one completed, one in progress)
3. Tool call `edit_file` running → success + diff on `notes.md`
4. Text: `"Done."`
5. Completed

`FakeAdapter::failing()` generates `Failed` instead, for error-path tests.

### 4. Health

`Available` | `Missing` | `Error { message }`. Fake probe is always `Available`.

### 5. Request

`GenerateRequest { session_id, user_text }`. No history, no worktree.

## Risks / Trade-offs

- [Callback is awkward for async OpenCode] → later change can add a streaming API without breaking the event types.
- [Fixed script ignores the prompt] → intentional for determinism.

## Migration Plan

None.

## Open Questions

None for this change.
