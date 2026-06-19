# Skill Discovery

## Purpose

Browse and search skills from skills.sh, view details (description, installs, author, repository), and install skills via `npx skills add <owner/repo>` — with a target selector to choose global or per-project installation.

## Requirements

### Requirement: Browse trending skills from skills.sh

The system SHALL fetch and display trending or popular skills from the skills.sh API.

#### Scenario: Fetch trending skills successfully
- **WHEN** the user navigates to the Discover tab in Skills settings
- **AND** the skills.sh API responds successfully
- **THEN** the system SHALL display a list of skills with name, description, install count, and author

#### Scenario: Skills.sh API is unavailable
- **WHEN** the skills.sh API request fails
- **THEN** the system SHALL display an error message "Could not load skills.sh" with a retry button
- **AND** previously cached results SHALL be displayed if available

### Requirement: Search skills from skills.sh

The system SHALL allow users to search skills.sh by keyword and display matching results.

#### Scenario: User searches for a skill
- **WHEN** the user types a search term in the Discover tab's search input
- **AND** submits the search
- **THEN** the system SHALL query the skills.sh API with the search term
- **AND** display matching skills sorted by relevance

#### Scenario: Search returns no results
- **WHEN** a search returns zero skills
- **THEN** the system SHALL display "No skills found for '<query>'" message

### Requirement: View skill details

The system SHALL display detailed information about a selected skill from skills.sh, including full description, install count, author, repository URL, and supported agents.

#### Scenario: User clicks on a skill in Discover
- **WHEN** the user clicks on a skill card in the Discover view
- **THEN** the system SHALL display a detail panel showing description, installs, author name, repository URL, and supported agents

### Requirement: Install skill from skills.sh

The system SHALL install a skill by executing `npx skills add <owner/repo>` in the main process and report progress and results via IPC. If a project target is specified, the system SHALL install into `<target>/.agents/skills/<name>/` instead.

#### Scenario: Successful global skill installation
- **WHEN** the user selects "Global" as the target and clicks "Install" on a skill in the Discover view
- **THEN** the main process SHALL execute `npx skills add <owner/repo>`
- **AND** the UI SHALL show a loading state with "Installing..."
- **AND** on success, the UI SHALL show "Installed" confirmation
- **AND** the installed skill SHALL appear in the Installed tab's "Global" group after refresh

#### Scenario: Successful per-project skill installation
- **WHEN** the user selects a registered project as the target and clicks "Install"
- **THEN** the main process SHALL clone the skill repository into `<project>/.agents/skills/<name>/`
- **AND** on success, the UI SHALL show "Installed in <project-name>"
- **AND** the installed skill SHALL appear in the Installed tab's "Project" group

#### Scenario: Skill installation fails
- **WHEN** `npx skills add` exits with a non-zero code
- **THEN** the system SHALL display the error message from the process output
- **AND** offer a retry button

#### Scenario: Installation times out
- **WHEN** `npx skills add` does not complete within 30 seconds
- **THEN** the main process SHALL kill the child process
- **AND** the UI SHALL display "Installation timed out" with a retry button

### Requirement: Installation queue prevents conflicts

The system SHALL queue installation requests and process them sequentially to prevent conflicts from parallel `npx` invocations.

#### Scenario: User installs multiple skills rapidly
- **WHEN** the user clicks "Install" on three different skills in quick succession
- **THEN** the system SHALL process installations one at a time
- **AND** display "Queued" for pending installations

### Requirement: IPC channel for skill installation

The system SHALL expose a `skills:install` IPC channel that accepts `{ ownerRepo: string, target?: string }` and returns installation result.

#### Scenario: Renderer initiates global skill installation
- **WHEN** the renderer calls `window.circulo.installSkill({ ownerRepo: "vercel-labs/skills" })`
- **THEN** the main process SHALL execute `npx skills add vercel-labs/skills`
- **AND** return `{ success: true }` or `{ success: false, error: "..." }`

#### Scenario: Renderer initiates per-project skill installation
- **WHEN** the renderer calls `window.circulo.installSkill({ ownerRepo: "vercel-labs/skills", target: "/path/to/project" })`
- **THEN** the main process SHALL clone the repository into `/path/to/project/.agents/skills/vercel-labs-skills/`
- **AND** return `{ success: true }` or `{ success: false, error: "..." }`
