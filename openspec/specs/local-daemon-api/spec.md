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

### Requirement: Health reports OpenCode status when using the real adapter

`GET /v1/health` MUST include OpenCode availability and version when the daemon runs the OpenCode adapter and the server responds to `GET /global/health`.

#### Scenario: OpenCode healthy

- **GIVEN** the OpenCode adapter and a healthy managed server
- **WHEN** the client calls `/v1/health`
- **THEN** the response includes OpenCode healthy true and a version string

#### Scenario: OpenCode unavailable

- **GIVEN** the OpenCode binary is missing or the server failed to start
- **WHEN** the client calls `/v1/health`
- **THEN** the response reflects OpenCode unavailable with a stable reason code mappable to locale copy

### Requirement: Agent list endpoint exposes registered providers

`GET /v1/agents` MUST return a JSON array of `AgentDescriptor` entries, one per registered provider. Each entry MUST include the `agent` enum (snake_case), an `available` boolean, and an optional `version` string.

#### Scenario: List agents in a single-provider build

- **GIVEN** a daemon build that only registers the OpenCode adapter
- **WHEN** the client calls `GET /v1/agents`
- **THEN** the response is a one-element array
- **AND** the element has `agent = "opencode"`, `available` reflects the live probe, and `version` is the OpenCode version when available

#### Scenario: Multi-provider build returns one entry per provider

- **GIVEN** a daemon build that registers both OpenCode and CommandCode
- **WHEN** the client calls `GET /v1/agents`
- **THEN** the response array contains one entry per registered provider
- **AND** each entry's `available` is independent of the others

#### Scenario: Disabled provider shows enabled = false

- **GIVEN** a daemon build with both providers, and CommandCode is in `UserPreferences.disabled_agents`
- **WHEN** the client calls `GET /v1/agents`
- **THEN** the CommandCode entry has `enabled = false`
- **AND** the OpenCode entry has `enabled = true`

### Requirement: Provider enable/disable endpoints

The daemon MUST expose `POST /v1/agents/{agent}/enable` and `POST /v1/agents/{agent}/disable`. Both endpoints update the `disabled_agents` set in `UserPreferences`, update the in-memory registry, and return the updated `PreferencesBody`. Disabling a provider MUST atomically migrate existing sessions of that agent to OpenCode.

#### Scenario: Disable with active sessions

- **GIVEN** the user has 3 sessions with `agent = command_code`
- **WHEN** the user POSTs to `/v1/agents/command_code/disable`
- **THEN** the response is 200 with the updated `PreferencesBody`
- **AND** all 3 sessions now have `agent = opencode`

#### Scenario: Enable is idempotent

- **GIVEN** CommandCode is already enabled
- **WHEN** the user POSTs to `/v1/agents/command_code/enable`
- **THEN** the response is 200 with no state change

### Requirement: AgentDisabled error code on session create

`POST /v1/sessions` MUST return 422 with `ErrorCode::AgentDisabled` when `body.agent` is in `UserPreferences.disabled_agents`. The error message MUST name the disabled provider.
