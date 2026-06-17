## ADDED Requirements

### Requirement: Chats folder new-chat button and auto-hide

The system SHALL display a "New Chat" icon button on hover over the "Chats" folder and SHALL hide the folder when it contains zero sessions.

#### Scenario: New Chat button appears on Chats folder hover
- **WHEN** the user hovers over the "Chats" folder in the sidebar's Threads section
- **THEN** a MessageCircle icon button SHALL appear next to the folder name
- **AND** clicking it SHALL navigate to the Chat view with the home directory pre-selected

#### Scenario: Chats folder disappears when empty
- **WHEN** all chat sessions under the Chats folder are deleted
- **THEN** the "Chats" folder SHALL no longer render in the sidebar

#### Scenario: Chats folder reappears when new chat is created
- **WHEN** a new chat session is created in the home directory
- **THEN** the "Chats" folder SHALL reappear in the sidebar's Threads section

### Requirement: Delete button on chat sessions inside Chats folder

The system SHALL display a delete button on hover for sessions inside the Chats folder.

#### Scenario: Delete button on chat session hover
- **WHEN** the user hovers over a session inside the Chats folder
- **THEN** a trash icon button SHALL appear
- **AND** clicking it SHALL delete the session
