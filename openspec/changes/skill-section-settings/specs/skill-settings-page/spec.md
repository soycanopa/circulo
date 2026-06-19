# Skill Settings Page

## Purpose

A dedicated Skills tab in the Settings UI that hosts both the skill inventory (Installed) and skill discovery (Discover) views.

## Requirements

### Requirement: Skills tab in Settings sidebar

The system SHALL add a "Skills" tab to the Settings page sidebar navigation.

#### Scenario: Skills tab appears in settings
- **WHEN** the user navigates to Settings
- **THEN** a "Skills" tab with a BookOpen icon SHALL be visible in the settings sidebar
- **AND** clicking it SHALL navigate to `/settings/skills`

#### Scenario: Skills tab is highlighted when active
- **WHEN** the user is on the Skills settings page
- **THEN** the "Skills" tab in the sidebar SHALL be visually highlighted

### Requirement: Installed skills view

The system SHALL display all installed skills grouped by origin (Global, Project, Cursor) in the Skills settings page.

#### Scenario: Installed tab shows skills by origin
- **WHEN** the user opens the Skills settings page
- **THEN** skills SHALL be displayed in collapsible groups labeled "Global", "Project", and "Cursor"
- **AND** each group SHALL show the count of skills it contains

#### Scenario: Global skills group
- **WHEN** global skills are installed
- **THEN** the "Global" group SHALL list each skill with name, description, and location path
- **AND** skills SHALL be sorted alphabetically by name

#### Scenario: Project skills group
- **WHEN** project-local skills exist
- **THEN** the "Project" group SHALL list each skill with name, description, and the project path it belongs to
- **AND** skills SHALL be grouped by project, then sorted alphabetically

#### Scenario: Cursor-migrated skills group
- **WHEN** Cursor-migrated skills exist
- **THEN** the "Cursor" group SHALL list those skills with a "Migrated from Cursor" indicator

#### Scenario: No skills installed
- **WHEN** no skills are found in any directory
- **THEN** the system SHALL display "No skills installed" with a link to the Discover tab

### Requirement: Refresh button

The system SHALL provide a way to manually refresh the installed skills list.

#### Scenario: User clicks refresh
- **WHEN** the user clicks the "Refresh" button on the Installed view
- **THEN** the system SHALL rescan all skill directories
- **AND** display a loading spinner while scanning
- **AND** update the list when complete

### Requirement: Discover tab

The system SHALL provide a "Discover" sub-tab within the Skills settings page for browsing and searching skills.sh.

#### Scenario: Navigate to Discover
- **WHEN** the user clicks the "Discover" sub-tab
- **THEN** the system SHALL display the skills.sh browser with trending skills
- **AND** a search input field at the top

#### Scenario: Search skills.sh from Discover
- **WHEN** the user types a query and presses Enter
- **THEN** the system SHALL display search results from skills.sh

### Requirement: Skill detail panel

The system SHALL display a detail panel when a skill is clicked in either the Installed or Discover view.

#### Scenario: Click installed skill shows details
- **WHEN** the user clicks an installed skill
- **THEN** a detail panel SHALL show the skill's name, full description, location path, and origin

#### Scenario: Click Discover skill shows details with install button
- **WHEN** the user clicks a skill in the Discover view
- **THEN** a detail panel SHALL show full description, installs, author, repository URL
- **AND** an "Install" button SHALL be visible
- **AND** a target selector dropdown SHALL be visible with "Global" (default) and registered project names as options

### Requirement: Install target selector

The system SHALL provide a dropdown in the Discover detail panel to select where the skill is installed: globally or in a specific registered project.

#### Scenario: Target selector defaults to Global
- **WHEN** the user opens the detail panel for a Discover skill
- **THEN** the target selector SHALL default to "Global"

#### Scenario: User selects a project target
- **WHEN** the user opens the target selector dropdown
- **THEN** all registered project names SHALL be listed as options after "Global"
- **AND** selecting a project updates the install target

#### Scenario: No registered projects
- **WHEN** no projects are registered
- **THEN** the target selector SHALL show only "Global" as an option

### Requirement: Responsive layout

The system SHALL layout the Skills page using the existing Settings section and row components for visual consistency.

#### Scenario: Layout matches existing settings
- **WHEN** viewing the Skills settings page
- **THEN** the page SHALL use `<SettingsSection>` for grouping
- **AND** use `<SettingsRow>` for individual skill entries
- **AND** match the visual style of General, Servers, and other settings pages

### Requirement: TanStack Router integration

The system SHALL register the Skills settings page as a child route of the settings route in the TanStack Router configuration.

#### Scenario: Navigate to skills settings via URL
- **WHEN** the URL is `/settings/skills`
- **THEN** the Skills settings component SHALL render
- **AND** the sidebar "Skills" tab SHALL be highlighted as active

#### Scenario: Navigate to skills settings via tab click
- **WHEN** the user clicks the "Skills" tab in settings sidebar
- **THEN** the URL SHALL update to `/settings/skills`
