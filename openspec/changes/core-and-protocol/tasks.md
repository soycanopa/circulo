## 1. Dependencies

- [x] 1.1 Add workspace deps: `serde`, `serde_json`, `uuid`, `time` with the features needed for JSON and RFC3339

## 2. Domain

- [x] 2.1 Implement core entities and enums (Project, Session, Message, parts, tool call, task, question, sidebar view)
- [x] 2.2 Implement project assignment lock via `first_send_at`
- [x] 2.3 Add domain tests: unassigned session, lock before/after first send, archived project, default sidebar view

## 3. Protocol

- [x] 3.1 Add `API_VERSION`, `ApiError`, and typed `ProtocolEvent`s listed in the spec
- [x] 3.2 Add protocol tests: event version, connected/completed roundtrip, assignment-locked error shape

## 4. Fixtures

- [x] 4.1 Add a mixed-parts assistant message JSON roundtrip test aligned with the project-definition example
