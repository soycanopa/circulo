# app-shell Specification

## Purpose

Provides Circulo’s native macOS window shell: custom title bar, sidebar traffic lights, collapse rail, and English strings from locale files.

## Requirements

### Requirement: Window uses a custom title bar

The app window MUST hide the default system title bar so chrome can live in the sidebar. Traffic lights MUST be positioned in the sidebar top bar, not in a native title bar.

#### Scenario: Title bar options

- **GIVEN** the window options used to open Circulo
- **WHEN** they are inspected
- **THEN** the title bar is configured as transparent
- **AND** a traffic-light position is set

### Requirement: Collapsed sidebar keeps window controls

The sidebar MUST collapse to a minimum rail. Traffic lights and the show/hide control MUST remain in that rail.

#### Scenario: Collapse width

- **GIVEN** the sidebar is expanded
- **WHEN** it is collapsed
- **THEN** its width is the rail width
- **AND** the rail width is greater than zero and smaller than the expanded width

### Requirement: UI copy comes from locales

Visible shell strings MUST be resolved from the English locale catalog. A missing key MUST fall back to English, then to the key itself.

#### Scenario: English hide label

- **GIVEN** the default English catalog
- **WHEN** `sidebar.hide` is requested
- **THEN** the value is a non-empty English string
- **AND** it is not the raw key

#### Scenario: Unknown key falls back

- **GIVEN** the English catalog
- **WHEN** a missing key is requested
- **THEN** the returned value is that key

### Requirement: Settings shows Circulo and OpenCode health

The General Settings section MUST display daemon health and, when available, OpenCode availability and version from `GET /v1/health`. The user MUST be able to retry the health check.

#### Scenario: OpenCode healthy

- **GIVEN** the daemon reports OpenCode available with a version
- **WHEN** the user opens Settings → General
- **THEN** OpenCode is shown as available with the version string

#### Scenario: OpenCode unavailable

- **GIVEN** the adapter cannot reach OpenCode
- **WHEN** the user opens Settings → General
- **THEN** OpenCode is shown as unavailable with human locale copy
- **AND** no stack trace is shown

### Requirement: Active projects can be archived or deleted from Settings

The Projects Settings section MUST list active projects and offer Archive and Delete. Delete MUST require an explicit confirmation step before calling the daemon.

#### Scenario: Archive removes project from sidebar

- **GIVEN** an active project with sessions
- **WHEN** the user archives it from Settings → Projects
- **THEN** the project and its sessions disappear from the main sidebar
- **AND** the project appears under Settings → Archived

#### Scenario: Delete requires confirmation

- **GIVEN** an active project
- **WHEN** the user chooses Delete
- **THEN** a confirmation affordance is shown before the delete request is sent

#### Scenario: Confirmed delete cascades

- **GIVEN** a confirmed delete for a project with sessions
- **WHEN** the delete succeeds
- **THEN** the project and its sessions are removed locally
- **AND** an open session from that project is deselected

### Requirement: Active projects can be renamed from Settings

The Projects Settings section MUST allow renaming an active project. The rename MUST call `PATCH /v1/projects/{id}` with the new name. After a successful rename, the project list and the Sidebar folder label MUST reflect the new name on the next refresh.

#### Scenario: Rename updates project and sidebar label

- **GIVEN** an active project with sessions
- **WHEN** the user renames it from Settings → Projects
- **THEN** the project name updates in the panel
- **AND** sessions belonging to that project show the new name in the Sidebar

#### Scenario: Empty rename name is rejected

- **GIVEN** the user is editing a project name in Settings → Projects
- **WHEN** they submit an empty or whitespace-only name
- **THEN** the rename is not sent
- **AND** a human error message is shown

### Requirement: Archived projects can be restored from Settings

The Archived Settings section MUST list archived projects and offer Restore without an extra confirmation dialog.

#### Scenario: Restore returns project to sidebar

- **GIVEN** an archived project
- **WHEN** the user restores it from Settings → Archived
- **THEN** the project becomes active again
- **AND** its sessions reappear in Today or Earlier on refresh

### Requirement: Settings exposes General, Projects, Archived, and Models

The Settings surface MUST expose exactly four navigation sections in this fixed order: **General**, **Projects**, **Archived**, **Models**. Each section routes to its dedicated panel. Removing, renaming, or reordering a section requires a new OpenSpec change.

#### Scenario: All four sections are visible

- **GIVEN** the user opens Settings
- **WHEN** the sidebar nav renders
- **THEN** General, Projects, Archived, and Models are listed in that order
- **AND** each section routes to its dedicated panel

#### Scenario: Models panel shows the model catalog

- **GIVEN** the user opens Settings → Models
- **WHEN** the panel renders
- **THEN** the catalog entries fetched from the daemon are listed
- **AND** the user can toggle a model's enabled state
- **AND** changes round-trip to the daemon

### Requirement: Settings exposes a Providers section

The Settings surface MUST include a `Providers` section as a fifth nav item. Each registered provider MUST be listed with a status (active / disabled / not installed) and a toggle. Disabling a provider MUST require an inline confirmation that explains the migration consequence. The toggle MUST be rejected with a human message when the change would leave zero providers enabled.

#### Scenario: Toggle row shows current state

- **GIVEN** the user opens Settings → Providers
- **WHEN** the panel renders
- **THEN** one row per registered provider is shown
- **AND** each row has a status badge and a toggle button whose label matches the current state (Disable when active, Enable when disabled)

#### Scenario: Disable requires confirmation

- **GIVEN** the user clicks Disable on a provider with active sessions
- **WHEN** the panel shows the confirm strip
- **THEN** the copy explains that existing sessions will move to OpenCode
- **AND** the user can confirm or cancel

#### Scenario: Last-enabled guard surfaces as human error

- **GIVEN** only one provider is currently enabled
- **WHEN** the user tries to disable it
- **THEN** the daemon returns 409 with a human message
- **AND** the toggle UI shows the message and the state is unchanged
