## ADDED Requirements

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

### Requirement: Archived projects can be restored from Settings

The Archived Settings section MUST list archived projects and offer Restore without an extra confirmation dialog.

#### Scenario: Restore returns project to sidebar

- **GIVEN** an archived project
- **WHEN** the user restores it from Settings → Archived
- **THEN** the project becomes active again
- **AND** its sessions reappear in Today or Earlier on refresh
