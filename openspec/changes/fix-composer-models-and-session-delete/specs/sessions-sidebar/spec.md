## ADDED Requirements

### Requirement: Deleting a session is optimistic in the UI

When the user confirms deleting a session, the sidebar MUST remove the session card immediately, without waiting for the daemon round trip. The daemon delete MUST run in the background. If the daemon delete fails, the app MUST surface the existing error banner and the session MUST reappear on the next session-list refresh.

#### Scenario: Card disappears immediately

- **GIVEN** a session card in the sidebar and a slow daemon
- **WHEN** the user confirms deleting that session
- **THEN** the card disappears from the sidebar immediately
- **AND** the UI stays responsive while the daemon delete is in flight

#### Scenario: Failed delete restores honesty

- **GIVEN** a session being deleted optimistically
- **WHEN** the daemon delete fails
- **THEN** the app shows the localized delete-failed error
- **AND** the session reappears when the session list refreshes

## MODIFIED Requirements

### Requirement: Today section lists sessions with activity today

The sidebar MUST show a **Today** section listing sessions whose activity timestamp (`last_message_at`, or `created_at` when no messages) falls on the current local calendar day. Each row MUST show the session title, folder name or localized **No folder**, and relative duration on the right.

#### Scenario: Unassigned session in Today

- **GIVEN** a session with no project and activity today
- **WHEN** the Today section renders that row
- **THEN** the folder label is the locale value for `session.without_folder` ("No folder" in `en`)
- **AND** relative duration appears on the right of the metadata row

#### Scenario: Assigned session in Today

- **GIVEN** a session with a project and activity today
- **WHEN** the Today section renders that row
- **THEN** the folder label is the project name
