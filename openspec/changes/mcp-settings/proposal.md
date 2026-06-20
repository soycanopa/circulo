## Why

Users configure MCP servers in OpenCode to extend agent capabilities (tools, resources, prompts), but Circulo currently provides zero visibility into these servers. Users must manually edit `opencode.json` to manage MCP configs, with no UI feedback on server status or health. This change brings MCP server management into Circulo's settings, giving users visibility, control, and a guided setup experience.

## What Changes

- New **MCP** settings tab that fetches and displays all configured MCP servers from OpenCode
- **Enable/disable toggle** for each MCP server with real-time status indicators (connected, disabled, failed, needs_auth)
- **Add MCP Server** flow: clicking "Add" opens a new chat where an OpenCode agent guides the user through configuring and installing an MCP server
- Server details: type (local/remote), command or URL, tool/resource count, connection status
- **OAuth authentication** controls for remote MCP servers that require it

## Capabilities

### New Capabilities

- `mcp-server-management`: View all MCP servers, toggle enable/disable, see real-time status, and launch guided setup via chat

### Modified Capabilities

None — this is an entirely new feature with no existing specs to modify.

## Impact

- **Renderer**: New `McpSettings` component, MCP-specific TanStack Query hooks, settings route
- **Settings page**: New "MCP" tab in `SettingsPage` sidebar and routes
- **OpenCode SDK**: New usage of `client.mcp.status()`, `client.mcp.connect()`, `client.mcp.disconnect()`, `client.mcp.add()`, and MCP-related types
- **Chat system**: Settings → chat navigation flow for guided MCP setup
- **IPC/Backend**: No changes needed — MCP operations go through the SDK client directly
