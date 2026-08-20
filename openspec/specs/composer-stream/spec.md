# composer-stream Specification

## Purpose

Lets the user send a message in a session, assign a project only at chat start, see replies stream live, and stop in-flight generations.

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

### Requirement: Replies stream live into the open transcript

While a turn is generating in the open session, the app MUST apply the daemon's session events to the transcript as they arrive, so assistant text, tool calls, and task lists appear incrementally without manual refresh. The transcript MUST equal the daemon's persisted state once the turn completes or fails.

#### Scenario: Text arrives incrementally

- **GIVEN** an open session with a generating turn
- **WHEN** the daemon emits `session.message.updated` events for the assistant message
- **THEN** the assistant content appears in the transcript progressively
- **AND** no manual refresh is needed

#### Scenario: Optimistic send

- **WHEN** the user sends a message
- **THEN** the draft clears immediately
- **AND** the user message appears in the transcript via `session.message.created`

#### Scenario: Failed turn is visible without fake success

- **GIVEN** an open session with a generating turn
- **WHEN** `session.message.failed` arrives
- **THEN** the transcript stops growing and reflects the failed state
- **AND** the composer stops showing the generating state

#### Scenario: Stream drop recovers with real data

- **GIVEN** an open session with an active event stream
- **WHEN** the stream drops
- **THEN** the app refetches the transcript and resubscribes
- **AND** no content is invented while disconnected

### Requirement: Transcript follows new content only when anchored

The transcript MUST keep following new content while the user is anchored at the bottom. When the user has scrolled up, the view MUST NOT jump, and a jump-to-latest affordance MUST be available while new content streams.

#### Scenario: Anchored at the bottom

- **GIVEN** the user is at the bottom of the transcript
- **WHEN** new content streams in
- **THEN** the view stays at the bottom

#### Scenario: Scrolled up during generation

- **GIVEN** the user has scrolled up
- **WHEN** new content streams in
- **THEN** the view does not move
- **AND** a jump-to-latest affordance is visible

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

### Requirement: Agent selector is visible only when more than one agent is available

The composer MUST render an `AgentSelector` chip only when `GET /v1/agents` reports more than one provider. The chip MUST list each `AgentDescriptor` and dispatch `PATCH /v1/sessions/{id}` with the chosen `agent` on selection. The chip MUST be disabled once the session's `first_send_at` is set.

#### Scenario: Single-agent build hides the selector

- **GIVEN** a daemon build with only OpenCode registered
- **WHEN** the composer renders
- **THEN** the AgentSelector is not visible

#### Scenario: Multi-agent build shows the selector pre-send

- **GIVEN** a daemon build with OpenCode and CommandCode registered
- **AND** an open session whose `first_send_at` is null
- **WHEN** the composer renders
- **THEN** the AgentSelector is visible
- **AND** choosing a different agent dispatches the PATCH

#### Scenario: Selector is disabled after first send

- **GIVEN** a session whose `first_send_at` is set
- **WHEN** the composer renders
- **THEN** the AgentSelector is visible but disabled
- **AND** displays the locked copy from the catalog

### Requirement: Model selection implies provider

When the user picks a model in the composer's model picker, the session's `agent` MUST be set to the picked entry's `agent` field. This is the user-facing way to switch providers in v0.3 (no separate provider selector). The app dispatches a single PATCH to `/v1/sessions/{id}` with both `composer_model_id` and `agent` set.

#### Scenario: Picking a model from the same provider

- **GIVEN** the session's current agent is `open_code` and the user picks another OpenCode model
- **WHEN** the picker click fires
- **THEN** the app PATCHes `composer_model_id` only
- **AND** the session's `agent` stays `open_code`

#### Scenario: Picking a model from a different provider

- **GIVEN** the session's current agent is `open_code` and the user picks a Command Code model
- **WHEN** the picker click fires
- **THEN** the app PATCHes both `composer_model_id` and `agent = command_code`
- **AND** the next send on this session dispatches to the Command Code adapter

#### Scenario: Picking a model from a disabled provider

- **GIVEN** the user has disabled Command Code in Settings → Providers
- **AND** the user picks a Command Code model
- **WHEN** the picker click fires
- **THEN** the daemon returns 422 with `ErrorCode::AgentDisabled`
- **AND** the UI surfaces the existing copy

### Requirement: Model picker has provider tabs

The model popover MUST show a vertical column of provider tabs on the left, one per Circulo provider that has at least one model in the visible catalog. Each tab MUST display only the provider's icon (no text label, no count). The tab has no horizontal padding so the column hugs the icons. The right column MUST render only the models whose `agent` matches the active tab. The default tab is the session's current `agent`; if that provider has no models, the picker falls back to the first available tab.

#### Scenario: Tabs reflect the catalog

- **GIVEN** the daemon's `/v1/models` returns 26 OpenCode + 56 CommandCode models
- **WHEN** the user opens the model picker
- **THEN** the popover shows two tabs (one per provider) with the provider's icon
- **AND** the right column lists only the models whose `agent` matches the active tab

#### Scenario: Switching tabs filters the list

- **GIVEN** the picker is open with the OpenCode tab active
- **WHEN** the user clicks the Command Code tab
- **THEN** the right column updates to show only Command Code models
- **AND** the tab visual state updates to reflect the active tab

#### Scenario: Disabling a provider removes its tab

- **GIVEN** the user disables Command Code in Settings → Providers
- **WHEN** the user re-opens the model picker
- **THEN** only the OpenCode tab is shown
- **AND** the right column shows only OpenCode models

### Requirement: Models panel sorts enabled first and shows provider icon

The Settings → Models panel MUST sort enabled models to the top of the list, with stable order by `(provider_name, name)` inside each group. Each row MUST display a small provider icon (OpenCode logo or CommandCode chevron) before the model name. The text badge for the provider is no longer used.

#### Scenario: Enabled models appear first

- **GIVEN** the user has enabled `claude-sonnet-5` (CommandCode) and `gpt-4o` (OpenCode)
- **WHEN** the user opens Settings → Models
- **THEN** the two enabled rows are at the top of the list
- **AND** the remaining disabled rows follow
