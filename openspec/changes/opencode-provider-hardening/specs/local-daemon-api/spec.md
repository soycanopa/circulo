## ADDED Requirements

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
