## Why

The previous change removed the "New Thread" hover button from projects in the sidebar, leaving only an archive button. Users need both actions available on hover — starting a new thread in a project AND archiving it. Additionally, the "Chats" synthetic folder lacks hover buttons for starting a new chat or removing chats, and when all chat sessions are deleted, the Chats folder remains visible as a ghost entry.

## What Changes

- **Restore New Thread button on projects**: Add back the `MessageCirclePlusIcon` hover button alongside the `ArchiveIcon` on project items, so users can both start a new thread and archive a project.
- **Chats folder hover buttons**: When hovering over the "Chats" folder in the sidebar, show a `MessageCircleIcon` that starts a new chat. Do NOT show an archive button for the Chats folder (cannot archive the home directory).
- **Chat sessions hover buttons**: When hovering over a chat session (inside the Chats folder), show a delete/remove button. The existing chat icon (new chat from session context) should also appear.
- **Chats folder auto-hide**: When all chat sessions are deleted and the `chatSessionIds` set becomes empty, the "Chats" folder SHALL disappear from the sidebar's Threads section.

## Capabilities

### New Capabilities

- `chats-folder-cleanup`: The Chats synthetic folder auto-hides when it contains zero sessions, and hover buttons provide new chat creation access.

### Modified Capabilities

- `chat-persistence`: Add requirements for Chats folder hover button (new chat icon) and auto-hide behavior when empty.
- `project-archive`: Modify the requirement that the archive button is the sole hover button — now both a new-thread button and an archive button appear on hover.

## Impact

- Affected code:
  - `apps/desktop/src/renderer/components/sidebar.tsx` — `ProjectFolder`: add back new-thread button alongside archive; `ProjectFolder` for Chats: add new-chat hover button; conditionally hide Chats when empty
