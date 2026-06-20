## ADDED Requirements

### Requirement: MCP settings tab is accessible from settings sidebar

The system SHALL provide an "MCP" tab in the settings sidebar navigation, placed after the "Providers" tab. Selecting it SHALL navigate to `/settings/mcp` and render the MCP server management view.

#### Scenario: User navigates to MCP settings via sidebar
- **WHEN** user clicks the "MCP" tab in the settings sidebar
- **THEN** the URL updates to `/settings/mcp`
- **THEN** the McpSettings component renders as the main content area
- **THEN** the "MCP" tab is visually highlighted as active

### Requirement: Display all configured MCP servers with status

The system SHALL fetch all MCP servers from OpenCode via SDK and display them in a list. Each server entry SHALL show its name, type (local/remote), current status, and a toggle to enable/disable.

#### Scenario: MCP servers are fetched and displayed
- **WHEN** the MCP settings page loads and the OpenCode server is running
- **THEN** all configured MCP servers are displayed with their names
- **THEN** each server shows a status indicator (connected, disabled, failed, or needs_auth)
- **THEN** each server shows a type badge ("Local" or "Remote")
- **THEN** each server shows an enable/disable toggle reflecting its current state

#### Scenario: No MCP servers are configured
- **WHEN** the MCP settings page loads and no MCP servers exist in OpenCode config
- **THEN** an empty state message is displayed with a prompt to add a server
- **THEN** the "Add MCP Server" button is prominently shown

#### Scenario: MCP server status updates in real time
- **WHEN** an external process changes an MCP server's status (e.g., server connects or disconnects)
- **THEN** the UI refreshes to show the updated status within the polling interval (5 seconds)

### Requirement: Enable and disable MCP servers via toggle

The system SHALL allow users to enable or disable an MCP server by toggling a switch control. Enabling SHALL call `client.mcp.connect(name)` and disabling SHALL call `client.mcp.disconnect(name)`.

#### Scenario: User enables a disabled MCP server
- **WHEN** user clicks the toggle on a disabled MCP server
- **THEN** the toggle moves to the "on" position immediately (optimistic update)
- **THEN** the SDK connect method is called for that server
- **THEN** on success, the status updates to "connected"
- **THEN** on failure, the toggle reverts to "off" and an error message is shown

#### Scenario: User disables a connected MCP server
- **WHEN** user clicks the toggle on a connected MCP server
- **THEN** the toggle moves to the "off" position immediately (optimistic update)
- **THEN** the SDK disconnect method is called for that server
- **THEN** on success, the status updates to "disabled"
- **THEN** on failure, the toggle reverts to "on" and an error message is shown

### Requirement: Add MCP server via chat flow

The system SHALL provide an "Add MCP Server" button that navigates to the new chat page with a pre-seeded prompt instructing the OpenCode agent to help configure and install an MCP server.

#### Scenario: User initiates add MCP server flow
- **WHEN** user clicks the "Add MCP Server" button
- **THEN** the app navigates to the new chat view (`/`)
- **THEN** the chat input is pre-filled with a prompt requesting MCP server setup assistance
- **THEN** a project directory selector is shown (standard new chat flow)

#### Scenario: User returns to MCP settings after adding a server via chat
- **WHEN** user completes the MCP setup chat and navigates back to settings
- **THEN** the newly added MCP server appears in the MCP settings list
- **THEN** its status reflects its current connection state

### Requirement: OAuth authentication controls for remote MCP servers

The system SHALL display OAuth-related controls for remote MCP servers that require authentication. For servers with `needs_auth` status, an "Authenticate" button SHALL be shown. For servers with `needs_client_registration` status, a "Register Client" button SHALL be shown.

#### Scenario: User authenticates an MCP server requiring OAuth
- **WHEN** an MCP server has `needs_auth` status
- **THEN** an "Authenticate" button is displayed next to the server
- **WHEN** user clicks "Authenticate"
- **THEN** the SDK OAuth start method is called for that server
- **THEN** the external browser opens to complete the OAuth flow

#### Scenario: User registers client for MCP server requiring registration
- **WHEN** an MCP server has `needs_client_registration` status
- **THEN** a "Register Client" button is displayed next to the server
- **WHEN** user clicks "Register Client"
- **THEN** the SDK OAuth authenticate method is called for that server
- **THEN** on failure, an error message is displayed

### Requirement: Handle OpenCode server offline state

The system SHALL detect when the OpenCode server is unreachable and display an appropriate message with a retry action.

#### Scenario: OpenCode server is not running when MCP settings loads
- **WHEN** the MCP settings page loads and the OpenCode server cannot be reached
- **THEN** a message "OpenCode server is offline" is displayed
- **THEN** a "Start Server" button is shown that triggers server startup
- **THEN** the MCP server list is not displayed

#### Scenario: OpenCode server becomes unreachable after initial load
- **WHEN** MCP servers are displayed and the OpenCode server goes offline
- **THEN** on the next poll cycle, an error state is shown
- **THEN** existing server data is preserved but marked as stale
- **THEN** toggles and action buttons are disabled

### Requirement: Display MCP server metadata

The system SHALL display additional metadata for each MCP server, including the command (for local servers) or URL (for remote servers), tool count, and resource count when available.

#### Scenario: View local MCP server details
- **WHEN** a local MCP server entry is displayed
- **THEN** the command and arguments are shown in a secondary text line
- **THEN** the working directory is shown if configured
- **THEN** timeout value is shown if non-default

#### Scenario: View remote MCP server details
- **WHEN** a remote MCP server entry is displayed
- **THEN** the URL is shown in a secondary text line
- **THEN** OAuth configuration status is indicated (configured/not configured/disabled)
