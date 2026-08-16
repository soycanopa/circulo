# composer-stream Specification

## Purpose

Lets the user send a message in a session, assign a project only at chat start, and see the resulting turn as a simple message list.

## Requirements

### Requirement: Composer sends only with a selected session

The send action MUST be disabled when there is no selected session or the draft is empty. When enabled, it MUST post the draft to the daemon and then show the new messages.

#### Scenario: Send with session

- **GIVEN** a selected session and a non-empty draft
- **WHEN** the user sends
- **THEN** a user message is stored
- **AND** an assistant message appears after the turn

#### Scenario: No session

- **GIVEN** no selected session
- **WHEN** the composer is shown
- **THEN** send is not available

### Requirement: Project picker locks after first send

The project folder can be chosen only before the first user message. After that the picker MUST be locked.

#### Scenario: Unlocked before first send

- **GIVEN** a session with no `first_send_at`
- **WHEN** the composer is shown
- **THEN** the project picker is enabled

#### Scenario: Locked after first send

- **GIVEN** a session with `first_send_at` set
- **WHEN** the composer is shown
- **THEN** the project picker is locked

### Requirement: Generating blocks a second send

While a send is in flight the composer MUST show a generating state and MUST NOT start another send.

#### Scenario: In flight

- **GIVEN** a send has started and not finished
- **WHEN** the composer is shown
- **THEN** it indicates generating
- **AND** another send is rejected
