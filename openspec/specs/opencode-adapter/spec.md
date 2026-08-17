## Purpose

Defines how Circulo's daemon obtains and manages a real OpenCode server, maps each Circulo session to one OpenCode session, streams normalized turn events, and surfaces failures as typed human errors.

## Requirements

### Requirement: Daemon owns a dedicated OpenCode server

The daemon MUST manage its own OpenCode server on a dedicated loopback port, distinct from any server the user may have started. If no healthy server responds on that port and an `opencode` binary can be located, the daemon MUST launch `opencode serve` bound to loopback on that port. If a healthy server already responds on that port, the daemon MUST reuse it instead of spawning another.

#### Scenario: Server is spawned when missing

- **GIVEN** no server responding on the dedicated port
- **AND** an `opencode` binary is locatable
- **WHEN** the daemon needs OpenCode
- **THEN** it launches `opencode serve` bound to `127.0.0.1` on the dedicated port
- **AND** the server becomes healthy within the startup timeout

#### Scenario: Healthy server is reused

- **GIVEN** a healthy server already responding on the dedicated port
- **WHEN** the daemon starts or needs OpenCode
- **THEN** no second server process is launched

#### Scenario: Foreign server is not attached

- **GIVEN** an OpenCode server running on a port other than the dedicated one
- **WHEN** the daemon needs OpenCode
- **THEN** the daemon does not connect to that foreign server

### Requirement: One Circulo session maps to one OpenCode session

Each Circulo session MUST be bound to at most one OpenCode session. The binding MUST be created when the session sends its first message and MUST be reused for every subsequent message in that session, including after daemon restarts.

#### Scenario: Binding is created lazily on first send

- **GIVEN** a Circulo session that has never sent a message
- **WHEN** its first message is sent
- **THEN** an OpenCode session is created and bound to the Circulo session before generation begins

#### Scenario: Binding is reused on subsequent sends

- **GIVEN** a Circulo session already bound to an OpenCode session
- **WHEN** another message is sent in that session
- **THEN** the message is delivered to the same OpenCode session
- **AND** no additional OpenCode session is created for it

#### Scenario: Binding survives a daemon restart

- **GIVEN** a Circulo session bound in a previous daemon run
- **WHEN** the daemon restarts and the session sends another message
- **THEN** the stored binding is used and OpenCode retains the conversation context

### Requirement: Turns stream normalized events from OpenCode

A turn MUST deliver OpenCode output as the normalized event sequence: incremental text deltas, tool-call updates, task lists, and completion or failure. OpenCode part or event types that Circulo does not model MUST be skipped without failing the turn.

#### Scenario: Text streams incrementally

- **GIVEN** a bound session and a healthy server
- **WHEN** a user message is sent
- **THEN** the assistant output arrives as incremental text deltas
- **AND** the turn ends in a completed event

#### Scenario: Unknown part type is skipped

- **GIVEN** the server emits a part type that Circulo does not model
- **WHEN** the turn is processed
- **THEN** the turn continues and completes
- **AND** the unknown part does not appear in the transcript

### Requirement: OpenCode failures surface as typed human errors

Every OpenCode failure mode — binary missing, server failing to start, unauthorized access, transport or stream failure mid-turn — MUST produce a typed adapter error with a stable kind and a human message from the locale catalog. A turn MUST NOT end in a scripted or empty success when OpenCode is not usable.

#### Scenario: Binary missing

- **GIVEN** no `opencode` binary can be located
- **WHEN** health is probed or a message is sent
- **THEN** the adapter reports unavailable with a human reason
- **AND** the assistant message, if any, fails without streaming fake text

#### Scenario: Server never becomes healthy

- **GIVEN** the binary exists but the server does not become healthy within the startup timeout
- **WHEN** the daemon probes it
- **THEN** the adapter reports unavailable with a human reason

#### Scenario: Stream drops mid-turn

- **GIVEN** a turn in progress
- **WHEN** the event stream fails before completion
- **THEN** the assistant message ends in a failed status with a stable error code
- **AND** the user-visible message is human and non-empty
