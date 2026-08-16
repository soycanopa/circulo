## 1. Persist helpers

- [x] 1.1 Add get/update project, update session title/status so PATCH/archive work

## 2. Protocol

- [x] 2.1 Add request DTOs and extra error codes needed by HTTP (`unavailable`, `internal`)

## 3. Daemon

- [x] 3.1 Reject non-loopback listen addresses
- [x] 3.2 Implement router: health, projects, sessions, messages, preferences, SSE
- [x] 3.3 Map fake adapter events into persisted assistant parts + protocol events
- [x] 3.4 Binary starts the server on the default or override loopback address

## 4. Tests

- [x] 4.1 Health on localhost
- [x] 4.2 Non-loopback bind rejected
- [x] 4.3 Create unassigned session
- [x] 4.4 Post message with fake turn persisted
- [x] 4.5 SSE first event is `server.connected`
- [x] 4.6 PATCH project after first send returns 409 locked
