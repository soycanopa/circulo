# Circulo — ACP mapping

Primary references (source of truth — do not invent flags or methods):

- [OpenCode ACP](https://opencode.ai/docs/acp/)
- [ACP Overview](https://agentclientprotocol.com/protocol/overview)
- [Session Setup](https://agentclientprotocol.com/protocol/session-setup)
- [Prompt Turn](https://agentclientprotocol.com/protocol/prompt-turn)

## Transport

Local agent subprocess, **JSON-RPC 2.0 over stdio**.

## OpenCode entrypoint (source of truth)

Per [OpenCode ACP docs](https://opencode.ai/docs/acp/) (Zed / JetBrains / nvim):

```json
{
  "command": "opencode",
  "args": ["acp"]
}
```

- Transport: JSON-RPC over **stdio**.
- **Do not** invent process flags for workspace — absolute `cwd` is sent on **`session/new`** per [ACP session-setup](https://agentclientprotocol.com/protocol/session-setup).
- Circulo resolves the `opencode` binary from PATH / `~/.opencode/bin` and spawns exactly `opencode acp`.
- Never use OpenCode HTTP/SSE for the primary chat path.

## Methods we implement (MVP)

| Direction | Method | Purpose |
|-----------|--------|---------|
| C→A | `initialize` | Version + capability negotiation |
| C→A | `session/new` | Create session with absolute `cwd` |
| C→A | `session/prompt` | User turn |
| C→A | `session/set_config_option` | Model/mode when offered |
| A→C | `session/update` | Stream chunks, tools, plans, usage |
| A→C | `session/request_permission` | Tool permission gate |
| C→A | `session/cancel` | User interrupt (Stop) |
| C→A | `session/load` | Resume saved chat on agent (when supported) |
| C→A | `session/close` | Close session before New Chat / delete |
| C→A | `session/set_config_option` | Model/mode when offered (per session) |

Multiple sessions can run concurrently against the same `opencode acp` process (configurable via `concurrent_sessions` capability; defaults to true).

## Connection generation & event gating

Each `open_project` increments a monotonic `connectionGeneration` counter on the Rust side. Every Tauri event (`agent:ready`, `acp:session_ready`, `acp:session_update`, `acp:permission_request`, `acp:config_options`, `acp:prompt_complete`, `acp:error`, `agent:disconnected`) carries this `connectionGeneration` in its payload. The frontend tracks `connectionGenerationAtom` and drops events whose generation no longer matches — preventing a previous OpenCode process from mutating the UI after a workspace switch or crash.

If `open_project` finishes before the frontend has registered listeners, `useBootstrapAgent` calls `reconcileFromStatus` so the bridge starts from the same source of truth the backend already published.

## Permission queue

Permission requests can fire concurrently on the same session. The frontend accumulates them in `pendingPermissionsAtom`; `activePermissionAtom` always holds the head of the queue. When the user responds, the queue advances. `respond_permission` validates that `optionId` belongs to the set returned by the agent and that `sessionId` matches the waiter; an invalid reply cancels the waiter instead of forwarding a forged option.

## Concurrent sessions & swap

Each `opencode acp` process can hold multiple ACP sessions at once. The Rust core tracks every session in `ActiveAgent.sessions: HashMap<String, SessionHandle>`; each entry has its own `prompt_in_flight` and a reference to the shared `cmd_tx`. `SendPrompt` / `SetConfigOption` / `CancelPrompt` carry an explicit `session_id`, so two chats in the same workspace can stream in parallel.

`AgentCapabilitiesDto.concurrent_sessions` defaults to `true` (OpenCode and most ACP agents support it). When `false`, `publish_or_create_session` falls back to the serial behavior (close-before-new).

`set_visible_session(sessionId)` swaps the active buffer without interrupting the previous session's RPC. The reducer mirrors the new session's `messages`/`streaming` into the legacy atoms (`messagesAtom`, `streamingTextAtom`, …) so the rest of the UI keeps reading a single buffer.

## Session storage map

The frontend mirrors the Rust session map via `sessionsAtom: Record<sessionId, SessionUiState>`. `visibleSessionIdAtom` points at the session currently bound to the composer; the sidebar shows a pulsing dot for any session with `promptInFlight: true` that is not the visible one. `disconnected` clears the map; `session_ready`/`prompt_complete`/`error` write per session.

## Lifecycle (must match ACP)

1. Spawn agent subprocess **once** (`opencode acp`).
2. `initialize` → agent process ready (UI: warm, no chat session).
3. Background prewarm (optional, Circulo): `session/new` with absolute `cwd` — **not** shown in UI until New Chat.
4. User New Chat → publish prewarmed session **or** `session/new` if none.
5. `session/prompt` → stream `session/update` → `session/prompt` result with `stopReason`.

**Never** spawn multiple agent processes for the same warm.

## Observed latency (OpenCode 1.18.5 on this machine)

These are **OpenCode process / LLM costs**, not Circulo RPC overhead:

| Step | Typical | Source |
|------|---------|--------|
| Cold process start | **~15–20 s** | `opencode --version` alone ~20s; same cost inside `initialize` |
| First `session/new` per process | **~6–10 s** | ACP: create context + connect MCP; OpenCode also loads `~/.config/opencode` MCP |
| Later `session/new` | **~20 ms** | Same process already warm |
| `session/prompt` turn | **~7–20 s** | LLM + tools; ends when agent returns `stopReason` |

Circulo mitigations (still protocol-correct):

1. Single-flight warm — one `opencode acp` process.
2. Eager warm at app start; **`open_project` is non-blocking** (UI never waits 15–20s on spawn).
3. Background `session/new` prewarm after initialize so New Chat is usually instant.
4. Stream `agent_message_chunk` immediately; do not wait for prompt RPC completion.
5. Optimistic assistant bubble + immutable message updates for Palot-like paint.

### Why Palot feels faster

Palot talks to a **already-running OpenCode HTTP/SSE server**. Circulo uses the documented ACP path (`opencode acp` subprocess). Cold `initialize` cost is OpenCode binary startup (~20s measured with `opencode --version` alone). After the process is warm, UI and New Chat should feel instant; prompt latency is LLM time-to-first-token + turn length.

### OpenCode config note

If `~/.config/opencode/opencode.json` enables local MCP servers (e.g. Pencil), OpenCode connects them during session setup. That adds to first `session/new` cost. Circulo does **not** invent flags to disable the user’s OpenCode config; disable MCP there if you want a lighter ACP agent.

## Conventions

- File paths are **absolute**.
- Property keys: `camelCase`.
- Discriminator strings: `snake_case`.
- User-readable text: Markdown.

## Multi-agent future

Agent binaries are selected via a registry (`id`, command, args, env). Chat UI and parser stay ACP-shaped so new agents plug in without rewriting the shell.
