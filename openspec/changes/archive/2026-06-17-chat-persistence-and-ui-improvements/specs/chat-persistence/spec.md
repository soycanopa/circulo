## ADDED Requirements

### Requirement: Chat sessions persist across app restarts

The system SHALL load sessions whose directory equals the user's home directory from the OpenCode server on app startup, so that chat sessions created in the Chat view remain visible in the sidebar's "Chats" folder after closing and reopening the app.

#### Scenario: Chat session appears after app restart
- **WHEN** the user creates a chat session via the Chat view
- **AND** the user closes and reopens the app
- **THEN** the session SHALL appear under the "Chats" folder in the sidebar's Threads section

#### Scenario: Home directory sessions load on startup
- **WHEN** the app starts and the home directory is resolved
- **AND** the server connection is established
- **THEN** the system SHALL fetch sessions for the home directory via `listSessions`
- **AND** the fetched sessions SHALL be merged into the Jotai session store

#### Scenario: Only chat sessions appear in Chats folder
- **WHEN** sessions exist in the home directory
- **THEN** only sessions whose directory matches the home directory SHALL be grouped under the "Chats" sidebar folder
- **AND** sessions in other project directories SHALL NOT appear under "Chats"

### Requirement: New Chat button in collapsed sidebar toolbar

The system SHALL display a "New Chat" icon button in the window controls area (next to the sidebar toggle and new thread buttons) when the sidebar is collapsed, allowing users to start a new chat without opening the sidebar.

#### Scenario: New Chat button visible when sidebar collapsed
- **WHEN** the sidebar is collapsed
- **THEN** a "New Chat" button with a MessageCircle icon SHALL be visible in the window controls area
- **AND** clicking it SHALL navigate to the Chat view with the home directory pre-selected

#### Scenario: New Chat button hidden when sidebar expanded
- **WHEN** the sidebar is expanded
- **THEN** the "New Chat" button in the window controls area SHALL NOT be visible

### Requirement: Chat icon on session hover in thread list

The system SHALL display a chat creation icon when hovering over a session item in the sidebar, allowing users to start a new chat in that session's project directory.

#### Scenario: Chat icon appears on session hover
- **WHEN** the user hovers over a session in the sidebar thread list
- **THEN** a MessageCircle icon button SHALL appear alongside the existing delete button
- **AND** clicking it SHALL navigate to the Chat view with the session's project directory pre-selected
