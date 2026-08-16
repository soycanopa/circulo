## Why

The daemon and UI cannot talk to an agent until there is a provider-agnostic adapter contract. A deterministic fake is required so later HTTP and GPUI work does not depend on a live OpenCode server.

## What Changes

- Define `AgentAdapter` in `circulo-adapter`: probe health, generate a turn, emit normalized events.
- Define `AdapterEvent` (text delta, tool call started/updated, task list, completed, failed) and `AdapterError` / `AdapterHealth`.
- Implement `circulo-adapter-fake` that always reports available and emits a fixed sequence: text, tasks, tool call + diff, completed.
- Tests do not require OpenCode or a display.

## Capabilities

### New Capabilities

- `agent-adapter`: Provider-agnostic generate/probe contract and a deterministic fake implementation.

### Modified Capabilities

- (none)

## Non-goals

- No OpenCode HTTP client.
- No daemon HTTP, no persistence calls from the adapter.
- No plugin loader.
- No worktree / working directory.

## Impact

- Crates: `circulo-adapter`, `circulo-adapter-fake`.
- Unblocks `local-daemon-api` and UI streaming against the fake.

## Open questions (not resolved here)

- How OpenCode is spawned remains open (TRD). The fake does not spawn anything.
