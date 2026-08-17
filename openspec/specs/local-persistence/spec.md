# local-persistence Specification

## Purpose

Stores Circulo projects, sessions, messages, and sidebar preference in a local SQLite database so the app survives restarts and enforces project lifecycle rules.

## Requirements

### Requirement: Sessions can be stored without a project

The store MUST persist a session with a null project id as the special Sessions folder. That folder MUST NOT be inserted as a Project row.

#### Scenario: Create unassigned session

- **GIVEN** an empty store
- **WHEN** a session is created with no project id
- **THEN** it can be loaded again with `project_id` null

### Requirement: Deleting a project deletes its sessions

When a project is deleted, the store MUST delete that project and every session (and those sessions’ messages) that referenced it.

#### Scenario: Cascade delete

- **GIVEN** a project with two sessions that have messages
- **WHEN** the project is deleted
- **THEN** the project is gone
- **AND** those sessions and messages are gone
- **AND** unassigned sessions remain

### Requirement: Archiving hides a project without deleting it

Archiving a project MUST keep its rows. Main session and group listings MUST omit archived projects and sessions whose project is archived. Restore MUST make them visible again.

#### Scenario: Archive then restore

- **GIVEN** a project with a session
- **WHEN** the project is archived
- **THEN** it appears in the archived-project list
- **AND** it does not appear in the active project list
- **AND** its session does not appear in the main session list
- **WHEN** the project is restored
- **THEN** the project and session appear in the main listings again

### Requirement: Project assignment is persisted and locked after first send

The store MUST persist `project_id` changes only when the domain allows it. After the first sent user message, a project change MUST fail.

#### Scenario: Lock after first user message

- **GIVEN** a stored session with no project
- **WHEN** a user message is saved
- **AND** a project assignment is attempted
- **THEN** the store rejects the assignment
- **AND** the session remains unassigned

### Requirement: Sidebar view preference defaults to sessions

The store MUST persist `sidebar.view` as `sessions` or `groups`. If the value is missing or not a known view, reads MUST return `sessions`.

#### Scenario: Missing preference

- **GIVEN** a new database
- **WHEN** the sidebar view is read
- **THEN** the result is `sessions`

#### Scenario: Corrupt preference

- **GIVEN** a stored value that is not `sessions` or `groups`
- **WHEN** the sidebar view is read
- **THEN** the result is `sessions`

### Requirement: Session title search

The store MUST find sessions whose title contains the query text, limited to sessions visible in the current listing rules (not under archived projects).

#### Scenario: Match by title

- **GIVEN** sessions titled “Landing copy” and “Budget notes”
- **WHEN** the store is searched for “land”
- **THEN** only “Landing copy” is returned

### Requirement: Agent session binding is persisted write-once

The store MUST persist an optional `opencode_session_id` per session. A session that has never talked to OpenCode MUST persist a null binding. Once a binding is stored, the store MUST reject replacing it with a different value.

#### Scenario: Store and reload a binding

- **GIVEN** a stored session whose binding is null
- **WHEN** an OpenCode session id is stored for it
- **THEN** reloading the session returns that id

#### Scenario: Unbound session stays null

- **GIVEN** a session created without sending a message
- **WHEN** it is stored and reloaded
- **THEN** its OpenCode session binding is null

#### Scenario: Binding cannot be replaced

- **GIVEN** a session already bound to an OpenCode session id
- **WHEN** a different id is stored for it
- **THEN** the store rejects the write
- **AND** the original binding is preserved
