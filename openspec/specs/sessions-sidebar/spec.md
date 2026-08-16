# sessions-sidebar Specification

## Purpose

Lets the user browse and create sessions and projects in the sidebar, switch Sessions/Groups views, and recover when the local daemon is not running.

## Requirements

### Requirement: Sessions view lists name, time, and project

In Sessions view the sidebar MUST list visible sessions. Each item MUST show the session title, a relative active time, and either the project name or the localized “No project” string.

#### Scenario: Unassigned session row

- **GIVEN** a visible session with no project
- **WHEN** the Sessions view renders that row
- **THEN** the project label is the locale value for `session.no_project`

### Requirement: Groups view nests sessions under projects

Groups view MUST list active projects and the sessions that belong to them. Sessions with no project MUST NOT appear. If there are no active projects, the view MUST show a New project action.

#### Scenario: Empty groups

- **GIVEN** no active projects
- **WHEN** Groups view is shown
- **THEN** a New project control is available

### Requirement: New session is unassigned and selected

Creating a session MUST call the daemon without a project id and select the new session.

#### Scenario: Create session

- **GIVEN** the sidebar
- **WHEN** the user activates New session
- **THEN** a session is created with no project
- **AND** it becomes the selected session

### Requirement: Sidebar view is remembered with Sessions fallback

The last Sessions/Groups choice MUST be stored on the daemon. If it cannot be read, the UI MUST show Sessions.

#### Scenario: Corrupt or missing preference

- **GIVEN** no usable stored view
- **WHEN** the sidebar loads
- **THEN** the active view is Sessions

### Requirement: Daemon down is honest

If the daemon cannot be reached after a single start attempt, the sidebar MUST show a localized error, not a crash or an empty list that looks like “no work”.

#### Scenario: Unreachable daemon

- **GIVEN** the daemon is not accepting connections
- **WHEN** the sidebar tries to load
- **THEN** the user sees the `sidebar.daemon_down` string
