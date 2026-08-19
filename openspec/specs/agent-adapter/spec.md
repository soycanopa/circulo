# agent-adapter Specification

## Purpose

Defines how Circulo talks to an agent provider through a normalized adapter, including a fake that works without OpenCode.

## Requirements

### Requirement: Adapter can report availability

An adapter MUST report whether the provider is available, missing, or in error, without talking to the Circulo UI.

#### Scenario: Fake is available

- **GIVEN** the fake adapter
- **WHEN** it is probed
- **THEN** health is available

### Requirement: Generate emits normalized events

Generating a turn MUST emit a sequence of normalized events that can include text deltas, tool-call updates, task lists, completion, or failure. The adapter MUST NOT persist Circulo sessions or render UI.

#### Scenario: Successful fake turn

- **GIVEN** the fake adapter
- **WHEN** a user turn is generated
- **THEN** the sequence includes at least one text delta
- **AND** a tool call that ends in success with a diff
- **AND** a task list
- **AND** a completed event
- **AND** no failed event

### Requirement: Failures are typed

An adapter failure MUST be an `AdapterError` with a stable kind (unavailable or failed) and a human message. It MUST NOT be a raw transport string as the only value.

#### Scenario: Fake can emit a failed turn when configured

- **GIVEN** a fake adapter configured to fail
- **WHEN** a user turn is generated
- **THEN** the result is a failed event or error
- **AND** the message is non-empty

### Requirement: Fake does not require OpenCode

The fake adapter MUST run without contacting OpenCode or the network.

#### Scenario: Offline generate

- **GIVEN** no OpenCode process
- **WHEN** the fake adapter generates a turn
- **THEN** it still emits the successful sequence

### Requirement: Registry dispatches by AgentType

The daemon MUST hold a registry of `Arc<dyn AgentAdapter>` keyed by `AgentType`. The daemon MUST dispatch `generate` calls to the adapter registered for the session's `agent`. A session whose `agent` is not registered MUST receive a 503 with a human error message at send time, not a crash.

#### Scenario: OpenCode session dispatches to OpenCode adapter

- **GIVEN** a session with `agent = opencode` and a registered OpenCode adapter
- **WHEN** the user sends a message
- **THEN** the daemon calls the OpenCode adapter's `generate`

#### Scenario: Unregistered agent returns 503

- **GIVEN** a session with `agent = command_code` and no CommandCode adapter registered
- **WHEN** the user sends a message
- **THEN** the daemon returns 503 with a human error

### Requirement: GET /v1/agents lists registered providers

The daemon MUST expose `GET /v1/agents` returning a list of `AgentDescriptor` entries, one per registered provider. Each descriptor MUST carry the `agent` enum, an `available` boolean, and an optional `version`.

#### Scenario: Only OpenCode is registered

- **GIVEN** a daemon build with the OpenCode adapter and no other providers
- **WHEN** the client calls `GET /v1/agents`
- **THEN** the response contains a single descriptor with `agent = opencode`
- **AND** `available` reflects OpenCode's current health probe
