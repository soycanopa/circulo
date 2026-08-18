## ADDED Requirements

### Requirement: User can stop an in-flight generation

While a turn is generating, the composer MUST offer a stop control that requests abort through the daemon. After stop, the composer MUST return to an editable state and the transcript MUST reflect the aborted turn without inventing a successful reply.

#### Scenario: Stop during a long reply

- **GIVEN** an open session with a generating assistant turn
- **WHEN** the user activates stop
- **THEN** the daemon aborts the OpenCode turn
- **AND** the composer is no longer read-only
- **AND** human copy explains that the reply was stopped

#### Scenario: Stop is unavailable when idle

- **GIVEN** no turn is generating
- **WHEN** the composer renders
- **THEN** stop is not shown (send remains the primary action)
