## ADDED Requirements

### Requirement: Threads section replaces Projects

The sidebar SHALL display a "Threads" section that replaces the current "Projects" section. Threads SHALL contain project folders and their associated sessions.

#### Scenario: Threads section is visible
- **WHEN** the sidebar renders and at least one project exists
- **THEN** a "Threads" group label is displayed instead of "Projects"

#### Scenario: Empty state when no threads exist
- **WHEN** no projects or chat sessions exist
- **THEN** the Threads section shows "No threads yet" with a muted helper message

### Requirement: Project folders use contextual icons

Project items in Threads SHALL use `FolderIcon` for local-only projects and `FolderGitIcon` for projects backed by a git repository.

#### Scenario: Local project shows folder icon
- **WHEN** a project is local (no git repository)
- **THEN** the project item displays `FolderIcon`

#### Scenario: Git project shows git folder icon
- **WHEN** a project has a git repository or is from GitHub
- **THEN** the project item displays `FolderGitIcon`

### Requirement: Click project name to expand

Clicking a project name in Threads SHALL toggle the expansion of its session list. No caret/chevron icon SHALL be displayed on project items.

#### Scenario: Click expands project sessions
- **WHEN** user clicks a collapsed project name
- **THEN** the project's sessions expand inline

#### Scenario: Click collapses project sessions
- **WHEN** user clicks an already-expanded project name
- **THEN** the project's sessions collapse

### Requirement: Archive project on hover

Hovering over a project in Threads SHALL reveal an archive button. Clicking the archive button SHALL permanently remove the project and all its conversations from visibility.

#### Scenario: Archive button appears on hover
- **WHEN** user hovers over a project item in Threads
- **THEN** an archive icon button (`ArchiveIcon`) appears on the project row

#### Scenario: Archive confirms removal
- **WHEN** user clicks the archive button on a project
- **THEN** the project and all its associated sessions are removed from the sidebar
- **THEN** the archived project no longer appears in Threads

#### Scenario: Archive button not visible without hover
- **WHEN** the mouse is not over a project item
- **THEN** the archive button is hidden

### Requirement: Session items use chat bubble icon

All session items in the sidebar SHALL replace their status-based icons with `MessageCircleIcon`.

#### Scenario: Session shows chat bubble
- **WHEN** a session item renders in any section (Active Now, project folder, Chat)
- **THEN** the leading icon is `MessageCircleIcon` instead of a status icon

#### Scenario: Running session shows animated chat bubble
- **WHEN** a session has status "running"
- **THEN** the `MessageCircleIcon` has the `animate-spin` class

#### Scenario: Unread indicator persists with new icon
- **WHEN** a session has new activity
- **THEN** a blue dot indicator is still shown overlaid on the chat bubble icon

### Requirement: Recent section removed

The sidebar SHALL NOT display a "Recent" sessions section. Sessions that are not active SHALL only appear within their project folder under Threads or under Chat.

#### Scenario: Recent section not rendered
- **WHEN** the sidebar renders with any number of non-active sessions
- **THEN** no "Recent" group label or recent session list is displayed

#### Scenario: Non-active sessions appear in project folders
- **WHEN** a project has non-active sessions
- **THEN** those sessions are accessible by expanding the project in Threads

### Requirement: Remove add-project plus button

The Threads section header SHALL NOT display a plus button for adding projects.

#### Scenario: Plus button not rendered in Threads header
- **WHEN** the Threads section renders
- **THEN** no `PlusIcon` button appears in the Threads group label area

### Requirement: Search and command palette remain in Threads header

The Threads section header SHALL retain the search toggle and command palette buttons.

#### Scenario: Search button available
- **WHEN** the Threads section renders
- **THEN** the search/filter toggle button is still present

#### Scenario: Command palette button available
- **WHEN** the Threads section renders
- **THEN** the command palette (`CommandIcon`) button is still present
