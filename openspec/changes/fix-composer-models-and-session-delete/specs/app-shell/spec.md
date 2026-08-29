## ADDED Requirements

### Requirement: Model catalog loads with bounded retry

The app MUST load the model catalog and the enabled-model preferences with a bounded retry policy. A failed or timed-out boot-time fetch MUST NOT leave the composer in the empty-catalog state for the rest of the process lifetime: whenever a session is activated and the in-memory catalog is empty, the app MUST re-attempt the fetch. The client-side timeout for the model catalog request MUST be long enough to cover a cold daemon-side catalog build (10 s), since the first catalog fetch may start agent servers. Fetch failures MUST NOT be silently discarded when they leave the composer without models.

#### Scenario: Boot fetch fails, session activation recovers

- **GIVEN** the app started while the daemon was still cold and the boot-time catalog fetch failed
- **WHEN** the user creates or selects a session afterwards
- **THEN** the app re-fetches the model catalog
- **AND** the model picker shows the enabled models once the fetch succeeds

#### Scenario: Catalog fetch keeps failing

- **GIVEN** the daemon cannot serve the model catalog
- **WHEN** the app exhausts its retry budget
- **THEN** the composer shows the localized empty-catalog label
- **AND** the next session activation triggers another fetch attempt

### Requirement: Touched session actions do not block the UI thread

Creating or deleting a session MUST NOT perform synchronous network IO on the UI thread as part of the flow. Post-action data refresh for these flows MUST go through the asynchronous refresh path.

#### Scenario: Delete keeps the UI responsive

- **WHEN** the user confirms deleting a session and the daemon is slow to respond
- **THEN** the window keeps rendering and accepts input while the delete round trip is in flight
