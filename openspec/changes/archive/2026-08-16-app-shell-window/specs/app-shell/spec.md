## Purpose

Provides Circulo’s native macOS window shell: custom title bar, sidebar traffic lights, collapse rail, and English strings from locale files.

## ADDED Requirements

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
