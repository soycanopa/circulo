## ADDED Requirements

### Requirement: Chats folder auto-hides when empty

The system SHALL hide the "Chats" synthetic folder from the sidebar's Threads section when it contains zero chat sessions.

#### Scenario: Chats folder disappears after all chats deleted
- **WHEN** all chat sessions are deleted from the Chats folder
- **THEN** the "Chats" folder SHALL no longer appear in the sidebar's Threads section

#### Scenario: Chats folder reappears when new chat is created
- **WHEN** a new chat session is created
- **THEN** the "Chats" folder SHALL reappear in the sidebar's Threads section with the new session

### Requirement: New Chat button on Chats folder hover

The system SHALL display a "New Chat" icon button when hovering over the "Chats" folder in the sidebar, allowing users to start a new chat in the home directory.

#### Scenario: New Chat button appears on Chats folder hover
- **WHEN** the user hovers over the "Chats" folder in the sidebar's Threads section
- **THEN** a MessageCircle icon button SHALL appear next to the folder name
- **AND** clicking it SHALL navigate to the Chat view with the home directory pre-selected

#### Scenario: No archive button on Chats folder
- **WHEN** the user hovers over the "Chats" folder
- **THEN** an archive/delete button SHALL NOT appear (the Chats folder is synthetic and cannot be archived)

### Requirement: Delete button on chat session hover

The system SHALL display a delete button when hovering over a chat session inside the Chats folder, allowing users to remove individual chat sessions.

#### Scenario: Delete button appears on chat session hover
- **WHEN** the user hovers over a session inside the Chats folder
- **THEN** a trash/delete icon button SHALL appear
- **AND** clicking it SHALL delete the session via the OpenCode server API
