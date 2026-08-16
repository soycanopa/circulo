# circulo-protocol Specification

## Purpose

Defines the versioned JSON contract between circulo-app and circulo-daemon so later HTTP/SSE work does not invent event shapes.

## Requirements

### Requirement: API version is explicit

Every protocol envelope that the daemon and app exchange MUST include `api_version`. The first version is `1`.

#### Scenario: Event carries version

- **GIVEN** a protocol event
- **WHEN** it is serialized
- **THEN** the JSON contains `"api_version": 1`

### Requirement: Typed session stream events

The protocol MUST define typed events: `server.connected`, `session.message.created`, `session.message.updated`, `session.part.appended`, `session.part.updated`, `session.tool_call.updated`, `session.message.completed`, `session.message.failed`.

#### Scenario: Connected event

- **GIVEN** a `server.connected` event
- **WHEN** it is serialized and deserialized
- **THEN** its type is `server.connected`
- **AND** `api_version` is 1

#### Scenario: Message completed event

- **GIVEN** a completed assistant message
- **WHEN** a `session.message.completed` event is serialized
- **THEN** it includes the session id and message id
- **AND** it roundtrips without loss

### Requirement: Errors have a stable code and a human message

A protocol error MUST include a stable machine `code` and a human-readable `message`. It MUST NOT be a raw OS or transport string as the only field.

#### Scenario: Project assignment locked

- **GIVEN** an attempt to change `project_id` after the first send
- **WHEN** the error is constructed
- **THEN** `code` is a stable identifier
- **AND** `message` is a non-empty human sentence

#### Scenario: Error JSON roundtrip

- **GIVEN** a protocol error
- **WHEN** it is serialized and deserialized
- **THEN** `code` and `message` match the original
