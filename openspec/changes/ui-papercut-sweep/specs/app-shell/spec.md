## ADDED Requirements

### Requirement: Session activation and creation do not block the UI thread

Selecting or creating a session MUST NOT perform synchronous network IO on the UI thread. Session activation clears the visible transcript immediately, shows a localized loading state while the transcript is in flight, and applies the fetched transcript through the asynchronous refresh path. All New-session entry points (Home card, Sidebar row, command palette) MUST share the same asynchronous creation path.

#### Scenario: Switching sessions with a cold daemon stays responsive

- **GIVEN** a session list with more than one session
- **WHEN** the user selects a session while the daemon is cold or slow
- **THEN** the window keeps rendering and accepting input during the load
- **AND** a localized loading indicator is shown instead of the empty-transcript state
- **AND** the transcript appears once the fetch completes

#### Scenario: Transcript load failure is visible

- **GIVEN** the daemon cannot serve the selected session's messages
- **WHEN** the user selects that session
- **THEN** the app shows an error instead of a silently empty transcript

#### Scenario: Sidebar and palette New session behave like Home

- **GIVEN** the Sidebar "New session" row or the palette "New session" command
- **WHEN** the user activates it
- **THEN** a session is created asynchronously and selected once created
- **AND** the UI remains responsive during the round trip

### Requirement: Sidebar error banner does not hide the session list

When an error state is set (daemon unreachable, send failure, rename failure, stream dropped, catalog failure), the sidebar MUST render the error as a banner above the Today/Earlier session sections; the session list MUST remain visible so navigation survives transient failures.

#### Scenario: Failed send keeps the session list usable

- **GIVEN** sessions exist in the sidebar
- **WHEN** a message send fails and an error banner is shown
- **THEN** the Today/Earlier sections and their session rows remain visible below the banner

### Requirement: UI copy comes from locales (no regression)

Every new visible string from session activation/loading MUST be resolved from the locale catalog; the empty-rename rejection for projects MUST use the catalog value for `settings.projects.rename_empty` and never a hardcoded literal.

#### Scenario: Empty project rename is rejected in English

- **GIVEN** the user submits an empty project name in Settings → Projects
- **WHEN** the rename is rejected
- **THEN** the shown message is the `settings.projects.rename_empty` catalog value

#### Scenario: Loading label is localized

- **GIVEN** a session whose transcript is loading
- **WHEN** the loading indicator renders
- **THEN** it shows the `messages.loading` catalog value
