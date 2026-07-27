# Circulo — Product Requirements Document (MVP)

## Problem

Coding agents such as OpenCode are powerful in the terminal but weak as multi-project desktop products. Palot shows the right product shape (workspace chrome, chat, permissions, diffs) but is built on Electron + OpenCode HTTP/SSE. Circulo is a **native ACP client**: one protocol, many agents.

## Product name & positioning

- **Name:** Circulo  
- **Positioning:** Desktop orchestrator for ACP-compatible coding agents  
- **Visual inspiration:** [Palot](https://github.com/ItsWendell/palot)  
- **Protocol:** [Agent Client Protocol](https://agentclientprotocol.com/) over stdio JSON-RPC  
- **First agent:** [OpenCode ACP](https://opencode.ai/docs/acp/) (`opencode acp`)

## Goals (MVP)

1. Open a project folder and chat with OpenCode via ACP.
2. Stream assistant text and tool calls in real time.
3. Permission gate: Approve / Deny (never auto-bypass).
4. `@` file mentions scoped to the project root.
5. Model / mode selectors when the agent exposes config options.
6. Dark, dense Palot-like shell (sidebar + chat + composer).

## Non-goals (MVP)

- Parallel multi-session streaming on one agent process  
- Automations / scheduler  
- Git commit / PR workflows  
- Claude Code / Cursor migration wizards  
- Remote agents / HTTP transport  
- Second agent binary (registry hooks only)

## User stories

1. Open Circulo → pick a project → send a message → see streaming reply.  
2. When the agent requests a tool permission, approve or deny inline.  
3. `@` a file so the agent receives that file as context.  
4. Switch model when the agent lists config options.  
5. Start **New Chat** and get a fresh ACP session without stuck UI.

## Success criteria

- Warm agent: new chat → first token typically under ~5s (not tens of seconds).  
- No “no active session” after a successful `session/new`.  
- No silent `session not found` on the first prompt of a new session.  
- Permission gate never auto-approved.

## Out of scope until post-MVP

Multi-session concurrency, automations, full diff review panel, terminal PTY, MCP admin UI, multi-agent binaries, ACP v2.
