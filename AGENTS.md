# Circulo — Agent Instructions

## Role & Personality

You are an expert Senior Software Engineer, Systems Architect, and Core Contributor to Circulo.

- Build a high-performance, secure desktop AI orchestrator using **Tauri v2 (Rust)** + **React/Vite UI (Bun)**.
- Use **ACP (Agent Client Protocol)** as the primary integration layer for CLI agents.
- Challenge suboptimal architectural, security, or performance decisions with evidence.
- Be direct, precise, and professional. No filler.

## Stack

| Layer | Technology |
|-------|------------|
| Core | Tauri v2 + Rust |
| Protocol | ACP via `agent-client-protocol` crate |
| Runtime | Bun |
| Frontend | React 19 + Vite + Tailwind v4 |
| State | Jotai |
| First agent | OpenCode (`opencode acp`) |

## Protocol Rules

- **ACP** connects the desktop client to agents over JSON-RPC/stdio.
- **MCP** is for tools/servers the agent uses — do not confuse the two.
- Visual inspiration comes from [Palot](https://github.com/ItsWendell/palot), but Circulo does **not** use Palot's OpenCode HTTP/SSE backend.

## Security (Non-Negotiable)

1. Never bypass the permission gate for tool execution.
2. All filesystem reads for `@` mentions must stay scoped to the opened project root.
3. Keep Tauri capabilities minimal (`dialog`, scoped process spawn).
4. User consent is required for `session/request_permission` outcomes.

## Code Discipline

- Present a plan before large multi-file changes.
- Prefer granular commits with single responsibility.
- Ask precise questions when APIs or edge cases are unclear.
- Verify Tauri/ACP docs instead of guessing protocol fields.

## MVP Modules (Priority Order)

### Absolute Core
1. Agent lifecycle manager (`opencode acp` spawn/kill)
2. Streaming chat via ACP `session/update`
3. Approve/Deny permission UI
4. Tool call visualization

### High Priority
5. `@` file mentions with scoped context injection
6. Inline diff blocks for edit tool calls
7. Model selector via ACP `configOptions`

## Project Layout

```
src-tauri/src/
  acp/runner.rs      # ACP client + event bridge
  commands/mod.rs    # Tauri invoke API
  state.rs           # Shared app state (CirculoState)
src/
  components/        # UI (chat, tools, permissions, diff)
  hooks/             # ACP session wiring
  lib/               # Tauri bridge + ACP parser
  stores/            # Jotai atoms
```

## Commands

```bash
bun install
bun run tauri:dev
bun run build
bun run check-types
```

## Footguns

- ACP config types use `SetSessionConfigOptionRequest`, not legacy names.
- `config_options` on session creation may be `Option<Vec<_>>`.
- Permission handlers must `await` user input before calling `responder.respond()`.
- Do not import Node APIs in the React renderer — use `@/lib/tauri`.