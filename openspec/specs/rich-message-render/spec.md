# rich-message-render Specification

## Purpose

Renders each message part so users can read Markdown, inspect tool work, review diffs, and follow task lists.

## Requirements

### Requirement: Text parts render as Markdown

A text part MUST show headings, lists, tables, block quotes, fenced code, links, and emphasis. Inline code MUST be visually distinct. A fenced code block MUST scroll horizontally instead of stretching the chat column.

#### Scenario: Heading and code

- **GIVEN** a text part with a heading and a fenced code block
- **WHEN** the message is shown
- **THEN** the heading is distinct from body text
- **AND** the code block is scrollable horizontally

### Requirement: Tool calls render as cards

A tool-call part MUST show a human-readable action name, a localized status, and a one-line context extracted from the input when a path, file, or query is present. Expanding the card MUST reveal the input and the output. Status MUST NOT be color-only.

#### Scenario: Successful file edit

- **GIVEN** a tool call named `edit_file` with input path `notes.md` and status success
- **WHEN** the card is shown collapsed
- **THEN** the user sees a readable name, a Ready status, and `notes.md`
- **WHEN** the user expands the card
- **THEN** the input and output are visible

### Requirement: Diff output is reviewable

A diff output MUST show the file path and +/- lines. A long diff MUST scroll inside the card so the rest of the transcript stays usable.

#### Scenario: Unified diff

- **GIVEN** a tool output with a file path and a unified diff that adds a line
- **WHEN** the output is shown
- **THEN** the path is visible
- **AND** the added line is marked as an addition

### Requirement: Task lists show ordered status

A task-list part MUST list tasks by `order` with title and localized status. Tasks MUST NOT be toggled in this change.

#### Scenario: Ordered tasks

- **GIVEN** two tasks with order 1 in progress and order 0 completed
- **WHEN** the list is shown
- **THEN** the completed task appears first
- **AND** each row shows its status as text

### Requirement: Unknown parts fail safely

A `Question` part or any part the UI does not understand MUST show the localized unsupported-block copy. The rest of the message MUST still render. The app MUST NOT crash.

#### Scenario: Question part

- **GIVEN** a message that includes a Question part
- **WHEN** the message is shown
- **THEN** that part shows the unsupported fallback
- **AND** other parts of the same message still render

### Requirement: Opaque reasoning is explained honestly

When an assistant message includes a reasoning part with no readable content (provider-encrypted or empty after the turn completes), the UI MUST show locale copy explaining that the provider hid the reasoning, instead of an empty expandable block.

#### Scenario: Encrypted reasoning after turn completes

- **GIVEN** a completed assistant message with a reasoning part whose content is empty
- **WHEN** the user expands reasoning
- **THEN** the UI shows human copy that reasoning is not available from the provider
- **AND** no fake or garbled text is shown
