## ADDED Requirements

### Requirement: Sidebar toggle button always present

The sidebar toggle button (`PanelLeftIcon`) SHALL always be visible in the window controls area, next to the macOS traffic lights.

#### Scenario: Toggle button always visible
- **WHEN** the app renders
- **THEN** the `PanelLeftIcon` toggle button is visible in the window controls position

### Requirement: New thread button only when sidebar collapsed

The new thread button (`MessageCirclePlusIcon`) SHALL only be visible when the sidebar is in a collapsed state. When the sidebar is expanded, the button SHALL be hidden.

#### Scenario: Button hidden when sidebar expanded
- **WHEN** the sidebar is expanded/open
- **THEN** the `MessageCirclePlusIcon` new-thread button is not visible in window controls

#### Scenario: Button shown when sidebar collapsed
- **WHEN** the sidebar is collapsed/closed
- **THEN** the `MessageCirclePlusIcon` new-thread button is visible in window controls

### Requirement: New thread button starts a new session

Clicking the new thread button (`MessageCirclePlusIcon`) when the sidebar is collapsed SHALL navigate to the home page to start a new thread.

#### Scenario: Click starts new thread
- **WHEN** user clicks the `MessageCirclePlusIcon` button while the sidebar is collapsed
- **THEN** navigation occurs to `/` (new thread page)

### Requirement: New thread sidebar item uses same icon

The "New Thread" item in the expanded sidebar SHALL also use `MessageCirclePlusIcon` instead of `PlusIcon`.

#### Scenario: New Thread shows chat-bubble-plus icon
- **WHEN** the sidebar is expanded and shows the "New Thread" menu item
- **THEN** the icon is `MessageCirclePlusIcon`

### Requirement: Server indicator moved next to Settings

The server indicator SHALL be displayed on the same row as the Settings button in the sidebar footer, aligned to the right side.

#### Scenario: Server indicator beside Settings
- **WHEN** the sidebar footer renders
- **THEN** the server indicator and Settings button appear on the same row
- **THEN** the server indicator is right-aligned

#### Scenario: Server indicator still shows status dot
- **WHEN** the server indicator renders beside Settings
- **THEN** the connection status dot (green/red) is still visible

#### Scenario: Server indicator still clickable
- **WHEN** user clicks the server indicator
- **THEN** the server switcher popover still opens
