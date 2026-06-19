# Skill Inventory

## Purpose

Enumerate installed skills from all configured directories (global, project-local, Cursor-migrated) and expose them to the renderer via IPC, with metadata and project associations.

## Requirements

### Requirement: Main process scans global skills directory

The system SHALL scan `~/.config/opencode/skills/` for installed skills by reading each subdirectory and extracting skill metadata from SKILL.md files.

#### Scenario: Global skills directory exists with skills
- **WHEN** the renderer requests installed skills
- **AND** `~/.config/opencode/skills/` exists and contains one or more subdirectories
- **THEN** the main process SHALL return a list of skills with name, description, location path, and origin set to "global"

#### Scenario: Global skills directory does not exist
- **WHEN** the renderer requests installed skills
- **AND** `~/.config/opencode/skills/` does not exist
- **THEN** the main process SHALL return an empty list without error

#### Scenario: Skill subdirectory has no SKILL.md
- **WHEN** scanning a skill subdirectory
- **AND** no SKILL.md file is found
- **THEN** the skill SHALL be included with its directory name as the name and "No description available" as the description

### Requirement: Main process scans project-local skills

The system SHALL scan registered project directories for locally-installed skills in their `.agents/skills/` and `.opencode/skills/` subdirectories.

#### Scenario: Project has local skills
- **WHEN** the renderer requests installed skills
- **AND** a registered project has `.agents/skills/` containing skill subdirectories
- **THEN** the main process SHALL return those skills with origin set to "project" and include the project path

#### Scenario: Project has no local skills
- **WHEN** the renderer requests installed skills
- **AND** a registered project has neither `.agents/skills/` nor `.opencode/skills/`
- **THEN** no project-local skills SHALL be returned for that project

### Requirement: Main process detects Cursor-migrated skills

The system SHALL scan `~/.cursor/skills/` for skills migrated from Cursor and report them with origin set to "cursor".

#### Scenario: Cursor skills directory exists
- **WHEN** the renderer requests installed skills
- **AND** `~/.cursor/skills/` exists and contains subdirectories
- **THEN** the main process SHALL return those skills with origin "cursor"

#### Scenario: Cursor skills directory does not exist
- **WHEN** the renderer requests installed skills
- **AND** `~/.cursor/skills/` does not exist
- **THEN** no cursor-origin skills SHALL be returned

### Requirement: Skill metadata extraction

The system SHALL extract skill name and description from each skill's SKILL.md file, falling back to the directory name if no metadata is found.

#### Scenario: SKILL.md has YAML frontmatter with description
- **WHEN** reading a skill's SKILL.md file
- **AND** the file contains YAML frontmatter with a `description` field
- **THEN** the skill's description SHALL be set to that value

#### Scenario: SKILL.md has no frontmatter
- **WHEN** reading a skill's SKILL.md file
- **AND** the file has no YAML frontmatter
- **THEN** the skill's description SHALL be the first non-empty, non-heading line of the file, truncated to 200 characters

### Requirement: IPC channel provides skill list to renderer

The system SHALL expose a `skills:list` IPC channel that the renderer can call to retrieve all installed skills grouped by origin.

#### Scenario: Renderer requests skill list
- **WHEN** the renderer calls `window.circulo.listSkills()`
- **THEN** the main process SHALL return an array of skill objects with `name`, `description`, `location`, `origin`, and optional `project` fields

#### Scenario: Rescan refreshes the skill list
- **WHEN** the renderer calls `window.circulo.listSkills()` after a skill installation
- **THEN** the main process SHALL rescan all directories and return the updated list

### Requirement: Skill object shape

Each skill object returned via IPC SHALL have the following shape:
- `name: string` — the skill name (directory name)
- `description: string` — extracted from SKILL.md or fallback
- `location: string` — absolute path to the skill directory
- `origin: "global" | "project" | "cursor"` — where the skill was installed from
- `project?: string` — project path if origin is "project"

#### Scenario: Global skill object
- **WHEN** a skill is found in `~/.config/opencode/skills/find-skills/`
- **THEN** the returned object SHALL have `name: "find-skills"`, `origin: "global"`, and no `project` field

#### Scenario: Project-local skill object
- **WHEN** a skill is found in `/path/to/project/.agents/skills/react-best-practices/`
- **THEN** the returned object SHALL have `name: "react-best-practices"`, `origin: "project"`, and `project: "/path/to/project"`
