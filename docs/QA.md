# Circulo — Manual QA checklist (MVP core)

Run after meaningful ACP/session changes. Requires OpenCode on `PATH` (`opencode acp`).

## Setup

```bash
bun install
bun run tauri dev
```

Optional: `RUST_LOG=circulo_lib=info` for timing logs.

---

## F0 — App launch (eager warm)


| Step                     | Expected                                                   |
| ------------------------ | ---------------------------------------------------------- |
| App opens                | UI visible immediately (no 15–20s block)                   |
| Sidebar footer           | Shows “OpenCode warming · ACP” then “OpenCode ready · ACP” |
| Center                   | “New Chat” CTA; composer disabled (“New Chat first…”)      |
| No session id in sidebar | “No session” until user clicks New Chat                    |


---



## F1 — Open project


| Step                               | Expected                                     |
| ---------------------------------- | -------------------------------------------- |
| Open project → pick a repo folder  | Returns immediately; workspace label updates |
| Wait for warm                      | Footer shows OpenCode ready                  |
| Session                            | Still “No session” until New Chat            |
| Pick `$HOME` / Desktop / Documents | Non-blocking warning banner (large folder)   |


---



## F4 — New chat


| Step                        | Expected                                            |
| --------------------------- | --------------------------------------------------- |
| Click New Chat (agent warm) | Usually fast (prewarmed session) or ≤10s first time |
| Sidebar                     | Real `sessionId` appears                            |
| Composer                    | Enabled                                             |
| Center                      | Empty chat, “Send a message…”                       |


---



## F2 — Send prompt


| Step                 | Expected                                   |
| -------------------- | ------------------------------------------ |
| Send a short message | User bubble + assistant streams text       |
| Status bar           | “Streaming…” then “Ready”                  |
| Second message       | Same session; no “no active session” error |
| Click **Stop** while streaming | Turn ends; composer returns to idle |

### @ file mentions

| Step                          | Expected                              |
| ----------------------------- | ------------------------------------- |
| Type `@` in composer          | File picker appears above input       |
| Filter with partial path      | Results update; ↑/↓ moves highlight   |
| Enter or Tab on a result      | `@relative/path` inserted in text     |
| Send with `@src/App.tsx` etc. | Agent responds using file context     |
| Invalid / escaped path        | Error banner (not silent failure)     |

---



## F3 — Permission


| Step                               | Expected                                |
| ---------------------------------- | --------------------------------------- |
| Trigger a tool that needs approval | Amber permission card in composer       |
| Approve                            | Agent continues; card disappears        |
| Deny (if offered)                  | Agent stops or reports failure; no hang |


---



## F4 again — New chat (same project)


| Step                             | Expected                          |
| -------------------------------- | --------------------------------- |
| New Chat while in a conversation | Messages cleared; new `sessionId` |
| Send message                     | Works on new session              |


---



## F5 — Switch project


| Step                    | Expected                                          |
| ----------------------- | ------------------------------------------------- |
| Open a different folder | Previous session cleared; workspace label updates |
| Re-open the **same** folder | Chat/session preserved (no wipe) |
| New Chat                | Session created for new cwd                       |
| Send message            | Agent context matches new project                 |


---



## Error paths


| Step                                     | Expected                                          |
| ---------------------------------------- | ------------------------------------------------- |
| OpenCode not on PATH (fresh install)     | Amber setup banner with install / `OPENCODE_BIN`  |
| Send without New Chat                    | Error: no active session                          |
| `OPENCODE_BIN` invalid / missing binary  | Clear error (banner or invoke error)              |
| New Chat before cold initialize finishes | Waits up to ~60s, then session or timeout message |


---



## F6 — Persistence & history

| Step | Expected |
|------|----------|
| New Chat + send messages | Chat appears in sidebar **Chats** list |
| Restart app | Saved chats still listed for workspace |
| Click saved chat | Transcript loads; banner says history / New Chat to continue |
| New Chat from history view | Live session; composer enabled |
| Open another project | **Recent** lists previous workspace |
| Settings (gear) | Shows agent command, chats folder, version |


---

## F7 — Polish (phase 3)

| Step | Expected | Verified |
|------|----------|----------|
| Assistant reply with fenced code blocks | Code renders in a highlighted monospace panel | [x] `Markdown` + `markdown.test.tsx` |
| Long chat while streaming | Message list auto-scrolls to the bottom | [x] `MessageList` `scrollIntoView` on messages/streaming |
| Tool output with diff content | Card uses diff styling (sky border) | [x] `ToolCallCard` sky border when `isDiffTool` |
| macOS window | Overlay titlebar; sidebar header is draggable | [x] `WindowChromeControls` + `data-tauri-drag-region` |
| `bun run test` | Parser unit tests pass | [x] 39 tests (incl. parser, markdown, diff-tools) |


---

## F8 — Session resume & delete (phase 4)

| Step | Expected | Verified |
|------|----------|----------|
| Click saved chat (agent supports load) | Transcript loads; composer enabled; can send follow-up | [x] `loadSession` + `reconcileSessionFromProjectStatus` in `App.tsx` |
| Click saved chat (load fails) | Read-only transcript + banner; New Chat still works | [x] fallback to `loadChatTranscript` + history banner |
| New Chat while in a live session | Previous session closed on agent; fresh session | [x] `createSession` / `closeSession` in runtime |
| Hover chat in sidebar → trash | Chat removed from list and disk | [x] `deleteChatTranscript` + `refreshPath` |
| Delete active live chat | Session cleared; composer disabled until New Chat | [x] `closeSession` before delete when active |


---

## F9 — Daily usability (phase 5)

| Step | Expected | Verified |
|------|----------|----------|
| Hover chat → pencil | Inline rename; title updates in sidebar | [x] inline edit in `AppSidebar` + `renameChatTranscript` |
| Export button / ⌘⇧E | Native save dialog writes `.md` file | [x] `exportTranscriptMarkdown` + `export-transcript.test.ts` |
| Assistant reply with `#` heading or `-` list | Renders headings and lists | [x] `Markdown` + `markdown.test.tsx` |
| `⌘N` / `⌘K` | New Chat / command palette | [x] `useAppShortcuts` |
| Empty workspace sidebar | Shows helpful empty-state copy | [x] “No general chats…” / “No chats in this project” |


---

## F10 — Diff panel (phase 6)

| Step | Expected | Verified |
|------|----------|----------|
| App bar **Diff** button (any screen) | Right sidebar opens, empty when no diffs | [x] `diffPanelOpenAtom` + `DiffPanel` in `AppShell` |
| Tool call with diff content | Click header → sidebar opens with full diff | [x] `onOpenDiff` in `MessageList` |
| Sidebar list | Shows all diff tools in current chat | [x] `collectDiffTools` + `diff-tools.test.ts` |
| Close (X) on sidebar | Panel closes | [x] `closeDiffPanel` |
| `⌘K` → Open Diff Panel | Palette toggles sidebar | [x] command palette item in `App.tsx` |


---

## F11 — MCP orchestrator + Skills + Usage (plan: MCP y Skills en Settings)

### MCP section

| Step | Expected |
|------|----------|
| Settings → MCP servers | Orchestrator `circulo-mcp` listed, built-in (not deletable), `on-demand` on |
| Toggle `auto-load` on a server | Server injected natively in next `session/new` with its full tool catalogue |
| Presets (Filesystem, GitHub, Brave…) | Clicking fills the guided form; validate spawns the server and lists its tools |
| Import | Reads `.mcp.json` (Claude Code) and `opencode.json` (mcp section); **read-only**, agent configs untouched |
| Delete a server | Removed from `~/.circulo/mcp.json` / project `.mcp.json`; `circulo-mcp` survives |
| `/mcp <name>` in composer | Inserts directive `mcp_load("<id>")` so the agent loads the server on-demand |
| Agent capabilities panel | Shows MCP stdio/http/sse + `terminal/*` delegation (runtime-observed) |

### Skills section

| Step | Expected |
|------|----------|
| Search a skill (e.g. "tdd") | Results from skills.sh; degraded banner if the endpoint changes |
| Search without a token | Uses the public `/api/search` endpoint (mode indicator shows "public") |
| Set a Vercel OIDC token | Search uses the official `/api/v1/skills/search` (semantic search); mode shows "authenticated" |
| Install to project | `SKILL.md` written under `.opencode/skills/<name>/` (inside project root) |
| Install globally | Written under `~/.config/opencode/skills/` |
| Install with a token | SKILL.md fetched from `/api/v1/skills/{id}` (handles well-known/domain sources) |
| Install without a token | SKILL.md resolved via the GitHub tree + raw URL |
| Delete | Removed from the chosen scope |

### skills.sh API (docs: https://skills.sh/docs/api)

- **Official endpoints (`/api/v1/*`)** require a Vercel OIDC token (`Authorization: Bearer`). Circulo stores it
  in `AppSettings.vercelOidcToken` (Settings > Skills) and uses it for search + install. The token is scoped
  to a Vercel project (OIDC Federation); without one the official API returns `authentication_required`.
- **Public fallback**: `GET /api/search` (undocumented but unauthenticated) returns `{ skills: [{id,
  skillId, name, installs, source}] }`. Install falls back to walking the GitHub tree for `SKILL.md`.
- Every search response reports `mode` ("authenticated" | "public") and `degraded: true` when the chosen
  path failed — the UI shows a banner instead of failing.

### Usage section

| Step | Expected |
|------|----------|
| Send prompts in a session | Sparkline grows from `usage_update` samples |
| Tool calls stream | "Tool output measured" accumulates bytes from `tool_call_update` |
| `compact_result` on terminal output | Savings counted; "Savings from compact_result" > 0 |
| `/mcp <name>` used | "MCP servers loaded" shows `<name> ×N` |

---

## F12 — Empirical capabilities matrix (fill as you test each agent)

`mcp_capabilities` comes from ACP `initialize`; `terminal/*` delegation is observed at runtime
(first `terminal/create` received). "Honors `mcp_servers`" = its tools are visible to the agent.

| Agent | Honors `mcp_servers` (stdio) | Exposes MCP tools | Delegates `terminal/*` | Notes |
|-------|------------------------------|-------------------|-------------------------|-------|
| OpenCode (`opencode acp`) | ✅ (verified: `sdk.mcp.add`) | ✅ | ❌ runs bash internally | No `terminal/*` → Fase 5 filter inactive |
| Grok (X ACP) | ⏳ to test | ⏳ to test | ✅ expected | `terminal/*` delegation → Fase 5 filter active |
| Cursor ACP | ⏳ to test | ⏳ to test | ⏳ to test | — |
| Generic ACP agent | ⏳ to test | ⏳ to test | ⏳ to test | Stdio mandatory per spec |

### Degradación limpia

- **Agent no honra `mcp_servers`**: circulo-mcp no aparece en el catálogo → el slash `/mcp` sigue
  mostrando el registro, pero el agente no puede llamar `mcp_load`. La sección MCP muestra
  `mcpStdio` (spec: siempre `true`) y el estado del orquestador; no hay crash.
- **skills.sh sin token OIDC**: se usa el endpoint público `/api/search` (buscar) + GitHub tree (instalar).
  La UI indica `mode: "public"`; no hay fallo.
- **skills.sh token inválido**: `search_skills_cmd` recibe 401 → devuelve `degraded: true` con mensaje
  claro; el instalador cae al GitHub tree. La UI muestra banner en vez de fallar.
- **Binario circulo-mcp ausente** (placeholder de `cargo check`): `resolve_circulo_mcp_binary`
  exige tamaño > 0; la UI avisa "Orchestrator binary not found" y desactiva la orquestación.
- **`terminal/output` sin filtro aplicable**: `filter_terminal_output` devuelve `applied: false`
  con passthrough (con el truncado de 256 KB como fallback); las métricas no se emiten.

---

## PRD success criteria

- [x] No “no active session” after successful New Chat + send  
- [x] No silent `session not found` on first prompt of a new session  
- [x] Permission gate never auto-approved  
- [x] Warm agent: New Chat → first token typically under ~5s (after process is warm)