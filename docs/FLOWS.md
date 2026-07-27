# Circulo — Core Flows (MVP)

## F1 — Open project

1. User selects a directory (native dialog).  
2. Rust validates/creates path if needed; spawns agent via registry (`opencode acp --cwd`).  
3. ACP `initialize` → emit `agent:ready` + capabilities.  
4. ACP `session/new` → emit `session:ready` (`sessionId`, `configOptions`).  
5. UI binds only that `sessionId` and enables the composer.

## F2 — Send prompt

1. Composer enabled only when session is ready.  
2. Optional `@` paths → Rust reads files under project root → content blocks.  
3. `session/prompt` with the active session id.  
4. Stream `session/update` notifications to the UI.  
5. Prompt RPC completes → status idle.

## F3 — Permission

1. Agent sends `session/request_permission`.  
2. UI shows options; Rust holds a oneshot until the user responds.  
3. Client responds with the selected option id.  
4. Agent continues or aborts the tool.

## F4 — New chat

1. User clicks New Chat.  
2. UI shows loading (no fake session id in storage).  
3. Rust `session/new` on the same agent process.  
4. On `session:ready`, clear messages and set the new id.  
5. Any pending first message is sent only after ready.

## F5 — Switch project

1. Shutdown previous agent process cleanly.  
2. Run F1 for the new path.

## State machine

```
Idle → OpeningProject → AgentReady → SessionReady → Generating ⇄ AwaitingPermission → Idle
```
