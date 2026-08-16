# local-daemon-api Specification

## Purpose

Exposes Circulo’s local HTTP and SSE API on loopback so the desktop app can manage sessions and stream a generated turn without talking to OpenCode itself.

## Requirements

### Requirement: Daemon binds only loopback

The daemon MUST listen only on a loopback address. It MUST refuse to start if asked to bind a non-loopback address.

#### Scenario: Health on localhost

- **GIVEN** a running daemon on 127.0.0.1
- **WHEN** a client calls `GET /v1/health`
- **THEN** the response is 200
- **AND** JSON includes `api_version` 1
- **AND** it reports daemon ok and adapter health

#### Scenario: Non-loopback bind is rejected

- **GIVEN** an address that is not loopback
- **WHEN** the daemon is asked to listen on that address
- **THEN** it does not start a server on that address

### Requirement: Unassigned session can be created over HTTP

`POST /v1/sessions` without a project MUST create a session with `project_id` null.

#### Scenario: Create session

- **GIVEN** a running daemon
- **WHEN** a client posts a new session without `project_id`
- **THEN** the session is returned with a null project id
- **AND** it can be fetched by id

### Requirement: Posting a message runs the adapter and persists the turn

`POST /v1/sessions/{id}/messages` MUST persist the user message, run the configured adapter, persist the assistant message, and finish with a completed or failed assistant message.

#### Scenario: Fake turn over HTTP

- **GIVEN** a session and the fake adapter
- **WHEN** a client posts `{ "content": "Hello" }`
- **THEN** listing messages includes a user message and an assistant message
- **AND** the assistant message has text and a successful tool-call part
- **AND** the assistant status is complete

### Requirement: SSE stream is typed

`GET /v1/sessions/{id}/events` MUST be an SSE stream whose first event is `server.connected` with `api_version` 1.

#### Scenario: Connected event

- **GIVEN** a running daemon and a session
- **WHEN** a client opens the session event stream
- **THEN** the first event type is `server.connected`
- **AND** `api_version` is 1

### Requirement: Assignment lock is visible over HTTP

Changing `project_id` after the first user message MUST fail with HTTP 409 and `project_assignment_locked`.

#### Scenario: Locked project patch

- **GIVEN** a session that already has a sent user message
- **WHEN** a client PATCHes a project id
- **THEN** the status is 409
- **AND** the error code is `project_assignment_locked`
