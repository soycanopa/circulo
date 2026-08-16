## Why

Users can open sessions but cannot send a message or see the turn. The composer and a plain message list are the next usable slice. Rich markdown/cards wait for the following change.

## What Changes

- Load messages when a session is selected.
- Composer draft: Enter sends, Shift+Enter inserts a newline.
- Project folder picker is enabled only before the first send; then it is locked.
- Send posts to the daemon, waits for the fake turn, and reloads messages.
- Generating state disables a second send.
- Empty/no-session composer stays disabled with locale copy.

## Capabilities

### New Capabilities

- `composer-stream`: Composer send, project lock, and a simple message list.

### Modified Capabilities

- (none)

## Non-goals

- No Markdown, ToolCallCard, or Diff viewer (next change).
- No SSE live paint (POST waits; then reload).
- No cancel.

## Impact

- `circulo-app` client: GET/POST messages, PATCH session project.
- Shell composer + message column.

## Open questions (not resolved here)

None that block this change. Enter-to-send is the convention recorded earlier.
