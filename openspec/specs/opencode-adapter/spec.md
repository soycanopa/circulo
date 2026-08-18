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

### Requirement: OpenCode server uses the session project directory when bound

When a Circulo session has an assigned project folder, the adapter MUST ensure OpenCode runs with that folder as its process working directory for turns in that session. When the session has no project (`Sessions` / unassigned), the adapter MUST use a documented safe default directory.

#### Scenario: Assigned session uses project path

- **GIVEN** a Circulo session bound to a project whose folder path exists on disk
- **WHEN** a turn runs for that session
- **THEN** OpenCode executes with that folder as cwd (or equivalent session path configuration supported by the server API)

#### Scenario: Unassigned session uses default cwd

- **GIVEN** a Circulo session with no project
- **WHEN** a turn runs
- **THEN** OpenCode uses the configured default cwd (not an arbitrary missing path)

### Requirement: Turns can be aborted in flight

The adapter MUST expose abort for a bound OpenCode session via `POST /session/:id/abort`. Abort MUST stop the current generation without deleting the OpenCode session or its history.

#### Scenario: Abort stops a long turn

- **GIVEN** a bound session with a turn in progress
- **WHEN** abort is requested
- **THEN** the adapter calls OpenCode abort
- **AND** the turn ends in a failed or cancelled state with human copy
- **AND** subsequent sends reuse the same binding

### Requirement: Mid-turn permission prompts can be answered

When OpenCode emits a permission request during a supervised turn, the adapter MUST surface it to the daemon and MUST apply the user's response via `POST /session/:id/permissions/:permissionID`.

#### Scenario: Supervised edit is approved

- **GIVEN** supervised permission mode and a pending permission event from OpenCode
- **WHEN** the user approves
- **THEN** the adapter posts the allow response
- **AND** the turn continues

#### Scenario: Permission is denied

- **GIVEN** a pending permission event
- **WHEN** the user denies
- **THEN** the adapter posts the deny response
- **AND** the turn ends or continues per OpenCode semantics without hanging Circulo

### Requirement: Deleting a Circulo session deletes the bound OpenCode session

When Circulo deletes a session that has a persisted `opencode_session_id`, the adapter MUST call `DELETE /session/:id` on OpenCode before or as part of local deletion. Failure to delete remotely MUST NOT block local delete but MUST be logged and surfaced once in human copy if observable.

#### Scenario: Bound session delete

- **GIVEN** a Circulo session with a stored OpenCode binding
- **WHEN** the session is deleted in Circulo
- **THEN** the adapter requests deletion of the OpenCode session

### Requirement: OpenCode health and version are probeable

The adapter MUST report OpenCode `{ healthy, version }` from `GET /global/health` when the server is reachable, in addition to the existing `/doc` identity probe.

#### Scenario: Healthy server reports version

- **GIVEN** a running Circulo-managed OpenCode server
- **WHEN** health is probed
- **THEN** the adapter returns healthy with a version string suitable for Settings or daemon health

### Requirement: Todo state can be reconciled after stream gaps

The adapter MUST be able to fetch `GET /session/:id/todo` for a bound session and map results to `TaskList` events when SSE recovery detects incomplete task state.

#### Scenario: Refetch after reconnect

- **GIVEN** a turn recovered after an SSE drop
- **WHEN** todo reconciliation runs
- **THEN** task cards match the OpenCode todo endpoint

### Requirement: Session title updates from OpenCode are observable

When OpenCode emits `session.updated` with a non-empty title for a bound session, the adapter MUST emit a normalized event the daemon can persist as the Circulo session title.

#### Scenario: Title arrives mid-turn

- **GIVEN** a bound session whose Circulo title is still default
- **WHEN** OpenCode updates the session title
- **THEN** the adapter forwards the title for persistence and sidebar refresh

### Requirement: Turns stream normalized events from OpenCode

A turn MUST deliver OpenCode output as the normalized event sequence: incremental text deltas, reasoning deltas (kept separate from reply text), tool-call updates, task lists, permission requests when applicable, title updates, and completion or failure. OpenCode part or event types that Circulo does not model MUST be skipped without failing the turn, except permission and title events which MUST be handled when present.

#### Scenario: Text streams incrementally

- **GIVEN** a bound session and a healthy server
- **WHEN** a user message is sent
- **THEN** the assistant output arrives as incremental text deltas
- **AND** the turn ends in a completed event

#### Scenario: Reasoning streams separately from reply text

- **GIVEN** a bound session and a model that emits reasoning parts
- **WHEN** a user message is sent
- **THEN** reasoning content is delivered as reasoning deltas
- **AND** reply text does not include reasoning content

#### Scenario: Unknown part type is skipped

- **GIVEN** the server emits a part type that Circulo does not model (and is not permission/title)
- **WHEN** the turn is processed
- **THEN** the turn continues and completes
- **AND** the unknown part does not appear in the reply transcript

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
