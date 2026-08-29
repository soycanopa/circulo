## ADDED Requirements

### Requirement: Session delete is local-first

`DELETE /v1/sessions/{id}` MUST delete the Circulo session row and cascade its messages first, and MUST return `204` without waiting for any agent-side cleanup. When the session has an agent session id, the daemon MUST schedule the agent-side session delete as best-effort background work: it MUST NOT block the HTTP response, and its failure MUST be logged without failing the request. A session that never reached an agent MUST delete without contacting the agent at all.

#### Scenario: Used session deletes fast while agent server is down

- **GIVEN** a session with messages and an agent session id, and the agent server is not running
- **WHEN** the client calls `DELETE /v1/sessions/{id}`
- **THEN** the daemon deletes the session row and its messages, and returns `204` without spawning the agent server to finish the response

#### Scenario: Agent-side cleanup fails after local delete

- **GIVEN** a session with an agent session id
- **WHEN** the background agent-side delete fails
- **THEN** the session is still deleted from Circulo
- **AND** the failure is logged and does not surface as an HTTP error

#### Scenario: Never-sent session deletes without agent contact

- **GIVEN** a session with no messages and no agent session id
- **WHEN** the client calls `DELETE /v1/sessions/{id}`
- **THEN** the daemon performs no agent-side call and returns `204`

## MODIFIED Requirements

### Requirement: Model catalog endpoint aggregates per-provider entries

`GET /v1/models` MUST return the union of model catalogs from every registered provider whose `enabled` flag is `true`. Each entry MUST carry the `agent` field so the client can dispatch implicitly when picking a model. The result is cached per provider with a TTL (default 5 minutes). The daemon MUST pre-warm the per-provider catalog cache for every enabled provider at startup, in the background, so the first client request does not pay the cold-catalog cost.

#### Scenario: Single-provider build

- **GIVEN** a daemon build with only OpenCode registered
- **WHEN** the client calls `GET /v1/models`
- **THEN** the response is the OpenCode model catalog
- **AND** every entry has `agent = open_code`

#### Scenario: Multi-provider build

- **GIVEN** a daemon build with both OpenCode and CommandCode registered
- **WHEN** the client calls `GET /v1/models`
- **THEN** the response contains OpenCode entries (with `agent = open_code`) and CommandCode entries (with `agent = command_code`)
- **AND** the entries are sorted by `(provider_name, name)`

#### Scenario: Disabled provider is excluded

- **GIVEN** the user has disabled CommandCode in Settings → Providers
- **WHEN** the client calls `GET /v1/models`
- **THEN** the response contains only OpenCode entries
- **AND** the CommandCode catalog is not fetched

#### Scenario: Startup pre-warms enabled providers

- **GIVEN** a daemon freshly started with OpenCode enabled
- **WHEN** the daemon becomes ready to serve
- **THEN** it pre-warms the OpenCode catalog cache in the background
- **AND** a `GET /v1/models` issued right after startup returns from the warm cache

#### Scenario: Disabled provider shows enabled = false

- **GIVEN** a daemon build with both providers, and CommandCode is in `UserPreferences.disabled_agents`
- **WHEN** the client calls `GET /v1/agents`
- **THEN** the CommandCode entry has `enabled = false`
- **AND** the OpenCode entry has `enabled = true`
