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
