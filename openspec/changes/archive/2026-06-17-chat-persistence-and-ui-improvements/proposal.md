## Why

Chat sessions disappear when the app is restarted because sessions created under the home directory are never loaded from the server on startup. Additionally, several UI gaps exist: missing archive/unlink for projects, no chat creation shortcuts from the sidebar widget area and thread list, and the "This Mac" server indicator in the sidebar footer lacks click interaction (no popover with server switching options).

## What Changes

- **Chat persistence**: Sessions created in the Chat view (home directory) persist across app restarts by loading them from the server on startup, appearing under the "Chats" folder in the sidebar.
- **New Chat button in window controls**: Adds a "New Chat" icon button next to the "New Thread" button in the collapsed-sidebar window controls area, so users can start a new chat without opening the sidebar.
- **Project archive button**: When hovering over a project in the sidebar's Threads section, an archive (folder-x) icon appears that removes/unlinks the project from the sidebar. Archived project IDs are persisted in localStorage.
- **Chat creation from thread list**: When hovering over a session in the sidebar, a chat icon appears that starts a new chat in that session's project directory.
- **Server indicator popover**: The server icon in the sidebar footer now opens the existing `ServerIndicator` popover with server switching, mDNS discovery, and health status when clicked.

## Capabilities

### New Capabilities

- `chat-persistence`: Sessions created in the Chat view (home directory scoped) persist across app restarts and appear under the sidebar's "Chats" folder.
- `project-archive`: Users can archive/unlink projects from the sidebar, hiding them while preserving data on the server.

### Modified Capabilities

<!-- None: these are additive UI features and initialization fixes. No existing spec-level requirements are changing. -->

## Impact

- Affected code:
  - `apps/desktop/src/renderer/components/sidebar-layout.tsx` — add new chat button to WindowControls
  - `apps/desktop/src/renderer/components/sidebar.tsx` — add archive button to ProjectFolder, add chat icon to SessionItem on hover, replace simple server icon with ServerIndicator component
  - `apps/desktop/src/renderer/components/server-indicator.tsx` — verify it works when rendered in sidebar footer (already functional, just not used there)
  - `apps/desktop/src/renderer/services/connection-manager.ts` — add session loading for home directory on startup
  - `apps/desktop/src/renderer/hooks/use-agents.ts` or derived atoms — ensure home directory sessions are properly grouped
