## ADDED Requirements

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
