## ADDED Requirements

### Requirement: Attach button visible in new-chat view
The new-chat screen SHALL render an attach button that opens the OS file picker for selecting files to attach to the initial message.

#### Scenario: User clicks attach button on new-chat screen
- **WHEN** user is on the new-chat screen and clicks the attach button
- **THEN** the OS file picker opens with accepted file types filter applied
- **AND** selected files appear as preview thumbnails/icons in the prompt area

#### Scenario: User sends message with attachments from new-chat
- **WHEN** user types a message, attaches files, and submits
- **THEN** the session is created with the message and attached files sent together

### Requirement: Attach button positioned before agent/model toolbar
The attach button SHALL appear to the left of the agent/model/variant selector toolbar in the `<PromptInputTools>` group in both chat-view and new-chat.

#### Scenario: Chat-view toolbar layout
- **WHEN** user is in an active chat session
- **THEN** the toolbar shows: [AttachButton] [AgentSelector] [ModelSelector] [VariantSelector] ... [SubmitButton]
- **AND** the attach button is visually grouped with the toolbar controls

#### Scenario: New-chat toolbar layout
- **WHEN** user is on the new-chat screen
- **THEN** the toolbar shows: [AttachButton] [AgentSelector] [ModelSelector] [VariantSelector]

### Requirement: Support text and code file attachments
The file attachment system SHALL accept text files (`.txt`), markdown files (`.md`), and common source code files (`.ts`, `.tsx`, `.js`, `.jsx`, `.json`, `.csv`, `.log`, `.yaml`, `.yml`, `.toml`, `.env`, `.html`, `.css`, `.xml`, `.py`, `.rb`, `.go`, `.rs`, `.java`, `.c`, `.h`, `.cpp`, `.sql`) in addition to the existing image (`.png`, `.jpg`, `.jpeg`, `.gif`, `.webp`) and PDF (`.pdf`) support.

#### Scenario: User attaches a markdown file
- **WHEN** user clicks attach and selects a `.md` file from the OS file picker
- **THEN** the file appears in the attachment preview with a file icon and filename
- **AND** the file is sent as a `FilePartInput` when the message is submitted

#### Scenario: User attaches a TypeScript file
- **WHEN** user drags and drops a `.ts` file onto the chat input
- **THEN** the file appears as an attachment preview
- **AND** the file is included in the message parts sent via the OpenCode SDK

#### Scenario: Model capability warning for unsupported types
- **WHEN** user attaches files and the selected model does not support images
- **THEN** a warning SHALL be displayed indicating the model may not support the attached content type

### Requirement: File type filtering in OS file picker
The `<PromptInput accept>` attribute SHALL include both MIME types and file extensions so that the OS native file dialog filters to relevant files.

#### Scenario: File picker shows appropriate file types
- **WHEN** user opens the attach file dialog
- **THEN** the OS file picker shows images, PDFs, and recognized text/code files by default

### Requirement: Consistent max file size across views
Both chat-view and new-chat SHALL use a max file size of 25MB for attachments.

#### Scenario: User attaches a file under 25MB
- **WHEN** user selects a 20MB file via the file picker
- **THEN** the file is accepted and shown in the attachment preview

#### Scenario: User attaches a file over 25MB
- **WHEN** user selects a 30MB file via the file picker
- **THEN** the file is rejected with an appropriate error message via the `onError` callback
