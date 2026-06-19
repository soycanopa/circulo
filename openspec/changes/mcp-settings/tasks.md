## 1. Route and tab scaffolding

- [x] 1.1 Add `"mcp"` to the `SettingsTab` type in `settings-page.tsx`
- [x] 1.2 Add `{ id: "mcp", label: "MCP", icon: PlugZap }` entry to the `tabs` array in `settings-page.tsx`
- [x] 1.3 Create `McpSettings` route in `router.tsx` (child of `settingsRoute`, path `"mcp"`)
- [x] 1.4 Create stub `McpSettings` component file at `components/settings/mcp-settings.tsx` with page title and empty state placeholder

## 2. MCP data hooks

- [x] 2.1 Add `useMcpStatus()` hook using TanStack Query with `client.mcp.status()` from `connection-manager.ts`, query key `["mcp", "status"]`, 5-second polling interval
- [x] 2.2 Import and use correct MCP types from `@opencode-ai/sdk/v2/client` (`McpStatus`, `McpLocalConfig`, `McpRemoteConfig`)
- [x] 2.3 Add `useMcpToggle()` mutation hook calling `client.mcp.connect(name)` / `client.mcp.disconnect(name)` with optimistic cache updates

## 3. MCP server list component

- [x] 3.1 Create `McpServerRow` sub-component rendering one server: name, type badge ("Local"/"Remote"), status badge (color-coded: green/connected, gray/disabled, red/failed, yellow/needs_auth), and toggle Switch
- [x] 3.2 Render server metadata in `McpServerRow`: command+args for local servers, URL for remote servers, as secondary muted text
- [x] 3.3 Wire `McpSettings` to use `useMcpStatus()` hook, map results through `McpServerRow` inside a `SettingsSection`

## 4. Enable/disable toggle with optimistic updates

- [x] 4.1 Wire Switch toggle to `useMcpToggle()` mutation in `McpServerRow`
- [x] 4.2 Implement optimistic update: toggle changes immediately, reverts on mutation error
- [x] 4.3 Show error toast on mutation failure using existing toast system
- [x] 4.4 Invalidate `["mcp", "status"]` query on mutation success to refresh status

## 5. Add MCP server chat flow

- [x] 5.1 Add search param (e.g., `prompt` or `mcpSetup`) to root route in `router.tsx` for pre-seeding chat input
- [x] 5.2 Read the search param in `NewChat` component and pre-fill the prompt textarea when present
- [x] 5.3 Add "Add MCP Server" button in `McpSettings` (top of page and in empty state) that navigates to `/?mcpSetup=true` with a pre-populated prompt text
- [x] 5.4 Clear the search param after the prompt is consumed so it doesn't persist on subsequent new chats

## 6. OAuth authentication controls

- [x] 6.1 Add `useMcpOAuth()` mutation hook calling `client.mcp.auth.start(name)` and `client.mcp.auth.authenticate(name)`
- [x] 6.2 Show "Authenticate" button in `McpServerRow` when status is `needs_auth`
- [x] 6.3 Show "Register Client" button in `McpServerRow` when status is `needs_client_registration`
- [x] 6.4 Display OAuth error messages on mutation failure

## 7. Empty, offline, and error states

- [x] 7.1 Add empty state when no MCP servers are configured: message + prominent "Add MCP Server" button
- [x] 7.2 Add offline state when OpenCode server is unreachable: "OpenCode server is offline" message + "Start Server" button calling `ensureOpenCode()`
- [x] 7.3 Add error state with retry button when `useMcpStatus()` query fails
- [x] 7.4 Disable all toggles and action buttons when server is offline or query is in error state

## 8. Polish

- [x] 8.1 Add loading skeletons (reuse `Skeleton` from `@circulo/ui`) while MCP status query is loading
- [x] 8.2 Add status indicator dots with color coding (green/connected, gray/disabled, red/failed, amber/needs_auth)
- [x] 8.3 Add type badge ("Local" / "Remote") next to each server name using `Badge` from `@circulo/ui`
- [x] 8.4 Verify lint, type-check, and formatting pass (`bun run lint`, `bun run check-types`)
