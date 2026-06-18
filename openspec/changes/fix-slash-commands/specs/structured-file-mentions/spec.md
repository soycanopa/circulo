## ADDED Requirements

### Requirement: File mention path sent as structured data
When a user `@mentions` a file in the chat, the prompt SHALL include both the text marker (`@filename`) and the resolved file path as structured data in the parts array sent to the server.

#### Scenario: User mentions a file and sends the prompt
- **WHEN** user `@mentions` `src/utils/helpers.ts` from the file picker and sends the prompt
- **THEN** the text part contains `@helpers.ts` as the user sees it
- **AND** the parts array includes a structured file reference with the resolved path `src/utils/helpers.ts`

#### Scenario: Multiple file mentions in one prompt
- **WHEN** user `@mentions` two files (`config.ts` and `types.ts`) in a single prompt and sends
- **THEN** both file paths are included as structured parts alongside the text

### Requirement: File mention path resolution uses full project-relative path
The structured file reference SHALL use the full project-relative path as resolved by `client.find.files()` and stored in the `MentionOption`.

#### Scenario: File mention resolves to correct project path
- **WHEN** user searches for `auth` and selects `src/auth/middleware.ts` from the mention popover
- **THEN** the mention state stores the full path `src/auth/middleware.ts`
- **AND** this path is used in the structured part when the prompt is sent

### Requirement: Backward compatibility for existing text-only mentions
Prompts without file mentions SHALL continue to work exactly as before — the text part is sent unchanged when no mentions are present.

#### Scenario: Prompt with no file mentions
- **WHEN** user types a message without any `@file` mentions and sends
- **THEN** the parts array contains only a text part with the raw message text
- **AND** no additional structured parts are added
