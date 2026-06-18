## ADDED Requirements

### Requirement: Skills with slash flag appear in popover
Skills from the OpenCode server with `slash: true` SHALL appear in the `SlashCommandPopover` alongside built-in and server commands.

#### Scenario: Slash-enabled skill shown in popover
- **WHEN** the slash command popover is opened
- **THEN** skills where `slash === true` are listed alongside other commands
- **AND** skills are visually distinguished (e.g., badge or icon) from built-in commands

#### Scenario: Non-slash skill not shown in popover
- **WHEN** the slash command popover is opened
- **THEN** skills where `slash` is `false` or `undefined` are excluded from the popover
- **AND** these skills remain accessible only via the `/skills` dialog

### Requirement: Skill type includes SDK fields
The local `Skill` interface in `skill-picker-dialog.tsx` SHALL include the `slash` and `content` fields matching the SDK's `SkillV2Info` type.

#### Scenario: Skill object has slash and content fields
- **WHEN** skills data is fetched from `client.app.skills()`
- **THEN** each skill object includes `name`, `description`, `location`, `slash`, and `content` fields
- **AND** the `slash` field is used to determine popover inclusion

### Requirement: Command execution passes agent and model overrides
When a slash command is executed via the popover, the command's `agent` and `model` fields from the SDK SHALL be passed to `client.session.command()`.

#### Scenario: Command with agent override
- **WHEN** user selects a command from the popover that has `agent: "review"` in its SDK definition
- **THEN** the `session.command()` call includes `agent: "review"`

#### Scenario: Command without overrides
- **WHEN** user selects a command from the popover that has no `agent` or `model` fields
- **THEN** `session.command()` is called without `agent` and `model` (undefined)
