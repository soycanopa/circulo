## ADDED Requirements

### Requirement: Session titles reflect OpenCode auto-titles when synced

When the daemon persists an auto-title from OpenCode for a session, the sidebar MUST show that title on the session card without requiring manual rename.

#### Scenario: Title updates after first message

- **GIVEN** a new session whose title was default
- **WHEN** OpenCode assigns a title and the daemon persists it
- **THEN** the sidebar card shows the new title on the next refresh or stream update
