## ADDED Requirements

### Requirement: Slash command popover on new-chat screen
The new-chat screen SHALL render a `SlashCommandPopover` that opens when the user types `/` at the start of the input, displaying available client and server commands (including skills with `slash: true`).

#### Scenario: User opens slash command popover on new-chat
- **WHEN** user types `/` at the beginning of the textarea on the new-chat screen
- **THEN** the slash command popover opens showing available commands
- **AND** the popover filters commands based on the typed query

#### Scenario: User selects a server command from the popover
- **WHEN** user selects a server command (e.g., `/init`) from the popover on new-chat
- **THEN** a session is created and the command is executed via `client.session.command()`

#### Scenario: User selects a skill from the popover
- **WHEN** user selects a skill command (e.g., `/brainstorming`) from the popover on new-chat
- **THEN** a session is created and the skill is executed as a command

### Requirement: Slash command execution on new-chat screen
The new-chat screen SHALL parse and execute slash commands typed directly into the textarea (without popover selection), creating a session if needed before executing.

#### Scenario: User types a slash command manually
- **WHEN** user types `/review my code` in the new-chat textarea and presses Enter
- **THEN** the command name `review` and arguments `my code` are parsed
- **AND** a session is created and the command is dispatched to `client.session.command()`

#### Scenario: Client-only commands are skipped on new-chat
- **WHEN** user types `/undo` on the new-chat screen and presses Enter
- **THEN** the command is not executed (no session exists to undo)
- **AND** a notification explains the command requires an active session

### Requirement: Skills dialog on new-chat screen
The new-chat screen SHALL render a `SkillPickerDialog` that opens when the user invokes the `/skills` command, displaying available skills from the OpenCode server.

#### Scenario: User browses skills on new-chat
- **WHEN** user selects `/skills` from the popover or types `/skills` and presses Enter on new-chat
- **THEN** the skill picker dialog opens showing available skills with name and description
- **AND** selecting a skill inserts `/{skillName} ` into the textarea
