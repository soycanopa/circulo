## ADDED Requirements

### Requirement: Deleting a session requires confirmation

The session context menu MUST stage the delete behind an explicit confirmation before the delete request is sent. The confirmation MUST use the `session.delete_confirm` catalog copy and offer Cancel and Confirm; cancelling (or switching sessions) MUST leave the session untouched. Once confirmed, deletion keeps its current semantics: the card is removed optimistically, the daemon delete runs in the background, and a failure surfaces through the error banner with the session restored on the next refresh.

#### Scenario: Delete is staged

- **GIVEN** a session with a transcript
- **WHEN** the user chooses Delete in the session context menu
- **THEN** a confirmation dialog is shown with `session.delete_confirm` copy
- **AND** no delete request has been sent yet

#### Scenario: Cancel leaves the session

- **GIVEN** a staged delete confirmation
- **WHEN** the user cancels (button, Escape, or outside click)
- **THEN** the session remains in the sidebar with its transcript intact

#### Scenario: Confirmed delete removes the card

- **GIVEN** a staged delete confirmation
- **WHEN** the user confirms
- **THEN** the session card disappears from the sidebar immediately
- **AND** the delete request is sent to the daemon once
