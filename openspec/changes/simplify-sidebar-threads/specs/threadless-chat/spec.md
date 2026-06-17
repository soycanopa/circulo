## ADDED Requirements

### Requirement: Chat section in sidebar

The sidebar SHALL display a "Chat" navigation item below the Threads section. This item SHALL start a new conversation scoped to the user's home/root directory.

#### Scenario: Chat item is visible
- **WHEN** the sidebar renders
- **THEN** a "Chat" item appears below the Threads section

#### Scenario: Clicking Chat navigates to chat view
- **WHEN** user clicks the "Chat" item
- **THEN** navigation occurs to a chat view scoped to the user's home directory

#### Scenario: Chat item icon
- **WHEN** the "Chat" item renders
- **THEN** it displays `MessageCirclePlusIcon`

### Requirement: Chat sessions grouped under Chats in Threads

Chat sessions started via the "Chat" feature SHALL appear as a special group labeled "Chats" within the Threads section, listing all free-floating conversations.

#### Scenario: Chat sessions appear as group in Threads
- **WHEN** one or more chat sessions exist
- **THEN** a "Chats" folder entry appears in the Threads section

#### Scenario: Expanding Chats shows all chat sessions
- **WHEN** user clicks to expand the "Chats" group
- **THEN** all free-floating chat sessions are listed as session items

#### Scenario: No Chats group when no chat sessions
- **WHEN** no chat sessions exist
- **THEN** the "Chats" group is hidden

### Requirement: Chat uses home directory as project root

Chat sessions SHALL use the operating system's home directory as their project root (`$HOME` on macOS/Linux, `%USERPROFILE%` on Windows).

#### Scenario: Chat session scoped to home directory
- **WHEN** a new chat session is created via the "Chat" feature
- **THEN** the session's working directory is the user's home directory

### Requirement: Chats persist across app restarts

Chat sessions SHALL persist and be discoverable across application restarts, appearing in the Chats group under Threads.

#### Scenario: Chat sessions survive restart
- **WHEN** the app is closed and reopened
- **THEN** previously created chat sessions still appear under the Chats group in Threads
