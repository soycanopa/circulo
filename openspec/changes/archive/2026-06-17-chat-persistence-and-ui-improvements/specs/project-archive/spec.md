## ADDED Requirements

### Requirement: Archive project from sidebar

The system SHALL allow users to archive (hide) a project from the sidebar via a hover-revealed button, storing the archived project ID in localStorage so it remains hidden across app restarts.

#### Scenario: Archive button appears on project hover
- **WHEN** the user hovers over a project in the sidebar's Threads section
- **THEN** an archive icon button (Archive or FolderX) SHALL appear next to the project name
- **AND** the button SHALL NOT appear for the synthetic "Chats" folder

#### Scenario: Archiving a project hides it from the sidebar
- **WHEN** the user clicks the archive button on a project
- **THEN** the project SHALL be removed from the visible project list in the sidebar
- **AND** the project ID SHALL be stored in localStorage under `circulo-archived-projects`
- **AND** the project's sessions and data on the server SHALL remain intact

#### Scenario: Archived projects stay hidden on restart
- **WHEN** a project has been archived
- **AND** the user closes and reopens the app
- **THEN** the archived project SHALL NOT appear in the sidebar

#### Scenario: Archived projects are re-discovered as new projects
- **WHEN** a project has been archived
- **AND** the user re-adds the same directory as a new project (e.g., via "Open folder")
- **THEN** the project SHALL reappear in the sidebar as a fresh project

### Requirement: Server indicator with popover in sidebar footer

The system SHALL render an interactive server indicator in the sidebar footer that opens a popover with server switching options, mDNS-discovered servers, health status, and a link to server settings.

#### Scenario: Server indicator shows connection status
- **WHEN** the sidebar is visible
- **THEN** the sidebar footer SHALL show a server icon with a colored status dot (green for connected, red for disconnected)
- **AND** the server name SHALL be displayed alongside the icon

#### Scenario: Clicking server indicator opens popover
- **WHEN** the user clicks the server indicator in the sidebar footer
- **THEN** a popover SHALL open showing the list of configured servers with health status dots
- **AND** the active server SHALL be highlighted
- **AND** clicking a different server SHALL switch to that server and close the popover

#### Scenario: Popover shows mDNS discovered servers
- **WHEN** the server indicator popover is open
- **AND** there are undiscovered servers on the local network via mDNS
- **THEN** those servers SHALL appear in a "Discovered on Network" section
- **AND** clicking a discovered server SHALL save it as a configured server

#### Scenario: Popover links to server settings
- **WHEN** the server indicator popover is open
- **THEN** a "Manage Servers..." option SHALL be visible
- **AND** clicking it SHALL navigate to the server settings page and close the popover
