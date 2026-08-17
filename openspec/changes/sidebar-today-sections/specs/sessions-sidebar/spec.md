## MODIFIED Requirements

### Requirement: Today section lists sessions with activity today

The sidebar MUST show a **Today** section listing sessions whose activity timestamp (`last_message_at`, or `created_at` when no messages) falls on the current local calendar day. Each row MUST show the session title, folder name or localized **Without Folder**, and relative duration on the right.

#### Scenario: Unassigned session in Today

- **GIVEN** a session with no project and activity today
- **WHEN** the Today section renders that row
- **THEN** the folder label is the locale value for `session.without_folder`
- **AND** relative duration appears on the right of the metadata row

#### Scenario: Assigned session in Today

- **GIVEN** a session with a project and activity today
- **WHEN** the Today section renders that row
- **THEN** the folder label is the project name

### Requirement: Earlier section lists older sessions

The sidebar MUST show an **Earlier** section below Today for sessions whose activity timestamp is before the current local calendar day. Rows use the same card layout as Today.

#### Scenario: Session from yesterday

- **GIVEN** a session whose activity was yesterday (local)
- **WHEN** the sidebar renders
- **THEN** the session appears under Earlier, not Today

### Requirement: Search filters both sections

Search MUST filter session titles in both Today and Earlier. Empty sections after filtering MUST be omitted.

#### Scenario: Search matches only Earlier

- **GIVEN** a search query matching only an Earlier session
- **WHEN** the sidebar renders
- **THEN** only the Earlier section is shown with matching rows

### Requirement: New session is unassigned and selected

Creating a session MUST call the daemon without a project id and select the new session.

#### Scenario: Create session

- **GIVEN** the sidebar
- **WHEN** the user activates New session
- **THEN** a session is created with no project
- **AND** it becomes the selected session

### Requirement: Daemon down is honest

If the daemon cannot be reached after a single start attempt, the sidebar MUST show a localized error, not a crash or an empty list that looks like “no work”.

#### Scenario: Unreachable daemon

- **GIVEN** the daemon is not accepting connections
- **WHEN** the sidebar tries to load
- **THEN** the user sees the `sidebar.daemon_down` string

## REMOVED Requirements

### Requirement: Sessions view lists name, time, and project

**Reason**: Replaced by Today/Earlier sections with updated copy.
**Migration**: Flat list removed; all sessions appear in Today or Earlier.

### Requirement: Groups view nests sessions under projects

**Reason**: Groups view removed.
**Migration**: Project grouping no longer in sidebar.

### Requirement: Sidebar view is remembered with Sessions fallback

**Reason**: No view switcher; preference removed from stack.
**Migration**: Stale `sidebar_view` row in SQLite is ignored.
