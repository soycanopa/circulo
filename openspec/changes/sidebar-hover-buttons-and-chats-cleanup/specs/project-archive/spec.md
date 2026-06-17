## MODIFIED Requirements

### Requirement: Archive project from sidebar

The system SHALL allow users to start a new thread or archive (hide) a project from the sidebar via hover-revealed buttons, storing the archived project ID in localStorage so it remains hidden across app restarts.

#### Scenario: New Thread and Archive buttons appear on project hover
- **WHEN** the user hovers over a project in the sidebar's Threads section
- **THEN** a new-thread icon button (MessageCirclePlus) SHALL appear next to the project name
- **AND** an archive icon button (Archive) SHALL appear next to the project name
- **AND** clicking the new-thread button SHALL navigate to the Chat view with the project directory pre-selected
- **AND** clicking the archive button SHALL archive the project

#### Scenario: No hover buttons on Chats folder
- **WHEN** the user hovers over the synthetic "Chats" folder
- **THEN** neither a new-thread nor an archive button SHALL appear (Chats uses its own hover buttons)

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
