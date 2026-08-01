# Circulo — Core Flows (MVP)

## F0 — App launch (eager warm)

1. Tauri `setup()` queues `open_project` on `~/.circulo/chats` (non-blocking).  
2. React bootstrap dedupes the same warm (Strict Mode safe).  
3. UI renders immediately; footer may show “warming”.  
4. ACP `initialize` → `agent:ready` (process warm, **no UI session yet**).  
5. Background `session/new` prewarm (optional, not published to UI).  
6. User clicks **New Chat** when ready (F4).

## F1 — Open project

1. User selects a directory (native dialog).  
2. Rust validates/creates path; shuts down previous agent if cwd changed.  
3. Spawns one agent process: `opencode acp` (stdio; **no** `--cwd` on the process).  
4. `open_project` returns immediately with updated `projectPath` (connected may still be false).  
5. ACP `initialize` → `agent:ready` + capabilities.  
6. Background `session/new` with absolute `cwd` (prewarm, hidden from UI).  
7. UI shows workspace; composer stays disabled until F4.

## F2 — Send prompt

1. Composer enabled only when `acp:session_ready` has fired for the active session.  
2. Optional `@` paths → Rust reads files under project root → content blocks.  
3. `session/prompt` with the active session id.  
4. Stream `session/update` (`agent_message_chunk`, tool calls) to the UI immediately.  
5. `acp:prompt_complete` → status idle.

## F3 — Permission

1. Agent sends `session/request_permission`.  
2. UI shows options; Rust holds a oneshot until the user responds.  
3. Client responds with the selected option id.  
4. Agent continues or aborts the tool.

## F4 — New chat

1. User clicks New Chat.  
2. UI shows loading (no fake session id).  
3. If agent still cold, Rust waits for `initialize` (up to 60s).  
4. Rust publishes prewarmed session **or** calls `session/new`.  
5. `acp:session_ready` → clear messages, bind `sessionId`, enable composer.

## F5 — Switch project

1. `open_project` sends `Shutdown` to previous process.  
2. Run F1 for the new path.  
3. UI clears session id and messages; waits for New Chat.

## State machine

```
Idle → OpeningProject → AgentReady → SessionReady → Generating ⇄ AwaitingPermission → Idle
```

`AgentReady` = `initialize` done. `SessionReady` = `session_ready_for_ui` true in Rust and `sessionId` in UI.
