# Circulo — Agent / contributor instructions

## Role

Senior engineer building a **Tauri v2 + Rust + React** desktop **ACP client**.

- Primary protocol: **ACP** (not MCP).  
- First agent: **OpenCode** (`opencode acp`).  
- Visual inspiration: Palot; **not** Palot’s Electron/OpenCode HTTP backend.

## Security (non-negotiable)

1. Never bypass the permission gate.  
2. `@` file reads stay under the opened project root.  
3. Minimal Tauri capabilities.  
4. User consent required for `session/request_permission`.

## Architecture rules

1. One visible session = one ACP session id.  
2. No optimistic/fake session ids in persistence.  
3. Prewarm `session/new` in background is OK; keep the session hidden until the first message (compose-first). Explicit New Chat still creates a fresh visible session.  
4. Never use `$HOME` as chats cwd — use `~/.circulo/chats`.  
5. Do not block the agent command loop on background pre-work.  
6. Present a plan before large multi-file changes.  
7. Prefer granular commits.

## Commands

```bash
bun install
bun run tauri dev
bun run build
bun run check-types
```

## Layout

```
src-tauri/src/   # Rust core (ACP runtime, commands, agent registry)
src/             # React UI
docs/            # PRD, TRD, UX, FLOWS, ACP
```
