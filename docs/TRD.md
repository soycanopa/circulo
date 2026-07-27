# Circulo — Technical Requirements Document (MVP)

## Stack

| Layer | Technology |
|-------|------------|
| Shell | Tauri v2 + Rust |
| Protocol | ACP v1 (`agent-client-protocol` crate) |
| Runtime | Bun |
| UI | React 19 + Vite + Tailwind v4 + shadcn/ui |
| State | Jotai (thin) |
| First agent | OpenCode (`opencode acp --cwd <project>`) |

## Architecture

```
React UI (Jotai)
    │ invoke / events
Tauri commands + AppState
    │ mpsc AgentCommand
acp::AgentRuntime
    │ JSON-RPC stdio
Agent process (registry → OpenCode first)
```

## ACP surface (MVP)

**Client → Agent:** `initialize`, `session/new`, `session/prompt`, optional `session/set_config_option`, optional `session/cancel`  

**Agent → Client:** `session/update`, `session/request_permission`  

Optional later: `session/load`, `session/list`, `session/close` when stable and needed.

## Agent registry

```rust
pub struct AgentDefinition {
    pub id: &'static str,
    pub label: &'static str,
    pub resolve_command: fn() -> Result<PathBuf, String>,
    pub args: fn(cwd: &Path) -> Vec<String>,
    pub env: fn() -> Vec<(String, String)>,
}
```

MVP implements OpenCode only; UI can show a single agent.

## Session model (simple by design)

```
Idle → OpeningProject → AgentReady → SessionReady → Generating ⇄ AwaitingPermission → Idle
```

Rules:

1. One agent process per open project (cwd fixed at spawn).  
2. One ACP `sessionId` is active in Rust and UI.  
3. New chat = `session/new`, then bind UI to that id before send.  
4. **No** placeholder / optimistic session ids written to transcript storage.  
5. **No** reserve/pre-warm sessions.  
6. Folder-less chats use `~/.circulo/chats`, never `$HOME`.

## Command loop invariant

Never block accepting `SendPrompt` / `NewSession` behind pre-warm or long index work.

## Security

- Permission handler awaits user input before responding.  
- `@` paths must resolve under the project root.  
- Minimal Tauri capabilities.

## Anti-patterns (do not reintroduce)

- Dual `activeSessionId` / `backendSessionId` without auto-heal  
- Reserve sessions that can go stale  
- Writing transcripts under `__optimistic_session__`  
- Using `$HOME` as agent cwd  
- Multiplexing concurrent prompts across sessions in MVP  

## Module layout (target)

```
src-tauri/src/
  state.rs
  agents/{mod.rs, opencode.rs}
  acp/{runtime.rs, events.rs, permissions.rs}
  commands/{project.rs, session.rs, chat.rs, permission.rs, fs.rs}

src/
  components/{layout,chat,permissions,tools,ui,settings}
  hooks/{use-acp-bridge,use-project,use-chat}
  lib/{tauri,acp-parser}
  stores/atoms.ts
  types/acp.ts
```
