# composer-stream Delta

## ADDED Requirements

### Requirement: Composer is a native multiline text field

The composer MUST use a dedicated GPUI input entity with visible focus, caret, and macOS IME support. Printable text MUST NOT rely on manual `KeyDown` character injection.

#### Scenario: Type with session selected

- **GIVEN** a selected session
- **WHEN** the user clicks the composer and types
- **THEN** characters appear in the field immediately

### Requirement: Enter and Shift+Enter use actions

Enter MUST submit when the draft is sendable. Shift+Enter MUST insert a newline.

#### Scenario: Newline

- **GIVEN** a selected session and non-empty draft
- **WHEN** the user presses Shift+Enter
- **THEN** a newline is inserted
- **AND** the message is not sent

### Requirement: Draft restores per session

When switching sessions, the composer MUST save the current draft and restore the draft for the selected session.

#### Scenario: Switch away and back

- **GIVEN** session A has draft "hello" and session B is selected with draft "world"
- **WHEN** the user selects session A again
- **THEN** the composer shows "hello"

### Requirement: Send error keeps composer text

If POST fails, the composer MUST still contain the message the user tried to send.

#### Scenario: POST error

- **GIVEN** a sendable draft
- **WHEN** POST returns an error
- **THEN** the draft text is restored in the composer

### Requirement: Project picker PATCHes on select

Before first send, choosing a project in the picker MUST PATCH the session project immediately.

#### Scenario: Pick project

- **GIVEN** an unlocked picker and a selected session
- **WHEN** the user picks a project
- **THEN** the session project is updated on the daemon

### Requirement: Composer grows with wrapped text and scrolls when tall

Long lines MUST wrap inside the input width. The input MUST grow vertically with content up to five visual lines, then scroll internally. When content exceeds five visual lines, an expand control MUST allow raising the visible cap to about ten lines.

#### Scenario: Long line wraps

- **GIVEN** a selected session
- **WHEN** the user types a long line without newlines
- **THEN** the text wraps inside the input
- **AND** the input height grows with wrapped lines up to five visible lines

#### Scenario: Expand tall drafts

- **GIVEN** a draft with more than five visual lines
- **WHEN** the user clicks the expand control
- **THEN** the input shows up to about ten visual lines before scrolling again
