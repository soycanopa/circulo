## ADDED Requirements

### Requirement: Skills appear in slash command popover
The system SHALL display available skills from both global and project-local locations in the slash command popover alongside server-defined commands.

#### Scenario: Typing slash shows skills
- **WHEN** the user types `/` in the chat input
- **THEN** the popover displays all available skills (global and local) mixed with server commands, with each skill marked with a `skill` badge

#### Scenario: Fuzzy filter matches skills
- **WHEN** the user types `/openspec` in the chat input
- **THEN** the popover filters skills whose name or description matches "openspec" using fuzzy matching (threshold 0.3)

#### Scenario: Selecting a skill executes it
- **WHEN** the user selects a skill from the slash command popover
- **THEN** the skill name is sent to the server via `client.session.command()` as a slash command (e.g., `/openspec-propose`)

### Requirement: Skills and commands are deduplicated
The system SHALL deduplicate entries when a skill and a command share the same name, giving priority to the server command.

#### Scenario: Skill with matching command name is not duplicated
- **WHEN** a skill named "review" exists and a server command named "review" also exists
- **THEN** the popover shows only the server command entry (which includes agent/model info)

### Requirement: Skills retain their badge in the popover
The system SHALL display a visual `skill` badge for all skill-sourced entries in the slash command popover.

#### Scenario: Skill badge is visible
- **WHEN** a skill entry appears in the popover
- **THEN** it displays a blue `skill` badge next to the entry, identical to skills with `slash: true` from the server

### Requirement: Skill picker dialog remains unchanged
The system SHALL keep the existing `SkillPickerDialog` functional and unchanged.

#### Scenario: Opening skill picker via /skills
- **WHEN** the user selects `/skills` from the popover or types `/skills`
- **THEN** the `SkillPickerDialog` opens with full skill descriptions and search functionality
