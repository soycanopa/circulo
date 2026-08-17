## ADDED Requirements

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
