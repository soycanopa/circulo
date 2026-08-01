# Circulo — Technical Requirements Document (MVP)

## Stack

| Layer | Technology |
|-------|------------|
| Shell | Tauri v2 + Rust |
| Protocol | ACP v1 (`agent-client-protocol` crate) |
| Runtime | Bun |
| UI | React 19 + Vite + Tailwind v4 |
| State | Jotai (thin) |
| First agent | OpenCode (`opencode acp` over stdio) |

## Architecture

```
React UI (Jotai)
    │ invoke / events
Tauri commands + CirculoState
    │ mpsc AgentCommand
acp::runtime (start_agent_connection)
    │ JSON-RPC stdio
Agent process (registry → OpenCode first)
```

## ACP surface (MVP)

**Client → Agent:** `initialize`, `session/new`, `session/prompt`, `session/set_config_option`  

**Agent → Client:** `session/update`, `session/request_permission`  

Optional later: `session/list`, client `fs/*`, `terminal/*`, elicitation.

Implemented post-MVP: `session/load`, `session/close`, `session/cancel`.

## OpenCode spawn (source of truth)

Per [OpenCode ACP docs](https://opencode.ai/docs/acp/):

- Command: `opencode`, args: `["acp"]`, transport: stdio.  
- **Do not** pass workspace via process flags.  
- Absolute `cwd` on **`session/new`** only ([session-setup](https://agentclientprotocol.com/protocol/session-setup)).

## Agent registry

MVP: `agents/mod.rs` resolves OpenCode binary (`PATH`, `~/.opencode/bin`, `OPENCODE_BIN`) and builds `AcpAgent::from_args(["opencode", "acp"])`.

## Session model (simple by design)

```
Idle → OpeningProject → AgentReady → SessionReady → Generating ⇄ AwaitingPermission → Idle
```

Rules:

1. One agent process per open workspace (single-flight warm).  
2. One ACP `sessionId` active in Rust and UI at a time.  
3. `session_ready_for_ui` — prewarm may hold a `sessionId` before the UI sees it.  
4. New chat = publish prewarm **or** `session/new`, then bind UI before send.  
5. **No** placeholder / optimistic session ids in persistence.  
6. Folder-less chats use `~/.circulo/chats`, never `$HOME`.  
7. Background `session/new` prewarm after `initialize` is allowed; must not emit `acp:session_ready` until New Chat.

## Warm / latency strategy

| Phase | Behavior |
|-------|----------|
| App launch | Eager `open_project(~/.circulo/chats)` — non-blocking |
| `open_project` | Returns immediately; `agent:ready` when `initialize` completes |
| After initialize | Optional background `session/new` (prewarm, hidden) |
| New Chat | Publish prewarm or fresh `session/new`; `create_session` waits if still cold |

See [ACP.md](./ACP.md) for measured OpenCode latencies.

## Command loop invariant

Never block the UI on cold `initialize`. `create_session` may wait for warm; `open_project` must not.

## Security

- Permission handler awaits user input before responding.  
- `@` paths must resolve under the project root (`canonicalize`).  
- Minimal Tauri capabilities (dialog, opener).

## Anti-patterns (do not reintroduce)

- Dual `activeSessionId` / `backendSessionId` without auto-heal  
- Publishing prewarmed sessions to the UI before New Chat  
- Writing transcripts under `__optimistic_session__`  
- Using `$HOME` as agent cwd  
- Spawning multiple `opencode acp` processes for the same warm  
- Multiplexing concurrent prompts across sessions in MVP  

## Module layout (current)

```
src-tauri/src/
  lib.rs, main.rs, state.rs, cli_resolve.rs
  agents/mod.rs
  acp/{mod.rs, runtime.rs}
  commands/mod.rs

src/
  App.tsx, main.tsx
  components/{layout,chat,permissions,tools}
  hooks/{use-acp-bridge,use-bootstrap}
  lib/{tauri,acp-parser,utils}
  stores/atoms.ts
  types/acp.ts
```
