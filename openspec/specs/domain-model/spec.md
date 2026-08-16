# domain-model Specification

## Purpose

Defines Circulo domain entities and assignment rules so every layer serializes the same projects, sessions, messages, and parts.

## Requirements

### Requirement: Project is optional on a session

A session MUST be allowed to exist with no project. A missing project means the session belongs to Circulo’s special Sessions folder, which is not itself a Project entity.

#### Scenario: New session is unassigned

- **GIVEN** a newly created session with no project id
- **WHEN** the session is serialized
- **THEN** `project_id` is null
- **AND** the session remains valid

#### Scenario: Assigned session names a user project

- **GIVEN** a session with a project id
- **WHEN** the session is serialized
- **THEN** `project_id` is that UUID
- **AND** that id refers to a user-created project, not a system Inbox row

### Requirement: Project assignment locks after first send

A session’s `project_id` MUST be changeable only before the first user message has been sent. After that send, assignment MUST be rejected.

#### Scenario: Assign before first send

- **GIVEN** a session with no sent user messages
- **WHEN** a project id is assigned
- **THEN** the assignment is accepted

#### Scenario: Assign after first send is rejected

- **GIVEN** a session that already has a sent user message
- **WHEN** a project id change is requested
- **THEN** the change is rejected

### Requirement: Structured message parts

A message MUST be a list of parts that can include text (Markdown), a tool call, a task list, or a question. Question parts MUST exist in the model.

#### Scenario: Assistant message with mixed parts

- **GIVEN** an assistant message containing text, a task list, and a tool call
- **WHEN** the message is serialized to JSON and read back
- **THEN** all three parts are present with the same values

#### Scenario: Tool call statuses

- **GIVEN** a tool call
- **WHEN** its status is set to pending, running, success, or error
- **THEN** the value roundtrips as that status

### Requirement: Project and session status

A project MUST be `active` or `archived`. A session MUST be `active`, `archived`, or `error`. Archiving MUST NOT be modeled as deletion.

#### Scenario: Archived project stays a project

- **GIVEN** a project with status archived
- **WHEN** it is serialized
- **THEN** status is `archived`
- **AND** its identity and name are preserved

### Requirement: Sidebar view preference

The domain MUST represent the sidebar view preference as `sessions` or `groups`, defaulting to `sessions` when missing.

#### Scenario: Default view

- **GIVEN** no stored preference
- **WHEN** the default view is requested
- **THEN** the value is `sessions`
