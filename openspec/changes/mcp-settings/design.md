## Context

Circulo is an Electron desktop app that manages OpenCode AI agent sessions. It already integrates with the OpenCode SDK (`@opencode-ai/sdk/v2`) for provider management, config, and session operations via TanStack Query. The settings UI has a tabbed layout (`SettingsPage`) with routes for general, servers, notifications, providers, worktrees, shortcuts, setup, and about.

The OpenCode SDK exposes a full MCP management API (`client.mcp.*`) including `status()`, `connect()`, `disconnect()`, `add()`, and OAuth methods — but none are currently used in the renderer. MCP server configs live in `opencode.json` and are invisible to Circulo users.

## Goals / Non-Goals

**Goals:**
- Show all configured MCP servers with live status (connected, disabled, failed, needs_auth)
- Let users enable/disable MCP servers with a toggle
- Display server metadata: type (local/remote), command/URL, tool count
- Provide an "Add MCP Server" button that opens a guided chat session
- Support OAuth authentication initiation for remote MCP servers

**Non-Goals:**
- In-place editing of MCP server configs (command, URL, env vars) — the chat flow handles configuration
- MCP tools/resources browser — this is a management view, not an explorer
- MCP server logs or detailed diagnostics
- Managing MCP for non-OpenCode providers (Claude Code, Cursor) — OpenCode only

## Decisions

### 1. Data fetching: TanStack Query via SDK client directly

Use `client.mcp.status()` with TanStack Query, following the existing pattern in `use-opencode-data.ts`. Use `queryKey: ["mcp", "status"]` and poll every 5 seconds for real-time status updates (MCP status can change independently of user actions via SSE events).

**Alternative considered**: SSE event-driven updates via `client.global.event()`. Rejected because MCP status events (`mcp.tools.changed`) only fire on tool changes, not on connect/disconnect/failure. Polling is simpler and covers all state transitions.

### 2. Component architecture: Follow existing settings patterns

`McpSettings` component mirrors `ProviderSettings` structure:
- `McpSettings` (page container) → `SettingsSection` + `SettingsRow` for the list
- `McpServerRow` sub-component renders one server: status badge, name, type badge, toggle, action buttons
- Reuse `@circulo/ui` components: `Switch`, `Badge`, `Button`, `Skeleton`

**Alternative considered**: Custom table layout. Rejected — `SettingsSection`/`SettingsRow` is the established pattern and provides consistent look.

### 3. Enable/disable: Call SDK connect/disconnect

Toggle calls `client.mcp.connect(name)` or `client.mcp.disconnect(name)` via a mutation. On success, invalidate the MCP status query to refresh. Show optimistic updates via TanStack Query's `onMutate`.

**Alternative considered**: Directly editing `opencode.json` config. Rejected — the SDK methods are the canonical API and handle config persistence internally.

### 4. "Add MCP Server" flow: Navigate to chat with pre-seeded prompt

When user clicks "Add MCP Server", navigate to the new chat page with a pre-filled prompt instructing the agent to help configure an MCP server. The agent uses `client.mcp.add()` SDK method or edits `opencode.json` directly — whichever it determines is appropriate.

Implementation: `navigate({ to: "/", search: { prompt: "/mcp add <user will specify>" } })` or use a dedicated URL param. The `NewChat` component reads the param and pre-fills the prompt input.

**Alternative considered**: Custom form/dialog for MCP config. Rejected — the user explicitly requested the chat-based flow, which also handles validation, suggestions, and error recovery better than a static form.

### 5. OAuth: Buttons to start auth flow

For servers with `needs_auth` status, show "Authenticate" button calling `client.mcp.auth.start(name)`. For servers with `needs_client_registration`, show "Register Client" button. These are simple mutation calls — no complex OAuth UI needed in settings.

### 6. Route and tab integration

Add `"mcp"` to `SettingsTab` type, add `{ id: "mcp", label: "MCP", icon: PlugZap }` to tabs array, add route in `router.tsx`:
```tsx
const mcpSettingsRoute = createRoute({
  getParentRoute: () => settingsRoute,
  path: "mcp",
  component: () => <McpSettings />,
})
```

## Risks / Trade-offs

- **OpenCode server must be running**: MCP status is fetched from the OpenCode server. If the server is stopped, the settings tab shows a "Server offline" state with a retry button. → Show clear offline messaging, offer to start server via existing `ensureOpenCode()` flow.
- **Polling overhead**: 5-second polling adds steady request load. → This is a single lightweight endpoint; consider increasing interval or switching to SSE if it becomes an issue.
- **Chat flow for add is non-deterministic**: The agent may not always produce a working MCP config. → The chat is a normal session — user can iterate with the agent. Consider adding a link back to MCP settings from the chat context.
- **OAuth browser flow**: `client.mcp.auth.start()` may need to open an external browser. → Use `setWindowOpenHandler` + `shell.openExternal()` pattern already established in main process.
