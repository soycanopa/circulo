## Context

Circulo's sidebar shows a "Chats" synthetic folder that groups sessions created from the Chat view (sessions whose directory equals the user's home directory). Currently, these sessions only appear because the SSE stream emits `session.created` events in real-time. On app restart, the SSE stream starts fresh with no backlog, and no code loads home-directory sessions from the server API. 

The project archive feature has its storage layer (`archiveProject` + localStorage) already implemented in `sidebar.tsx`, but the `onArchive` callback passed to `ProjectFolder` is never rendered as a UI element. The hover button in `ProjectFolder` currently shows a `MessageCirclePlusIcon` that navigates to the project page — unrelated to archiving.

The `ServerIndicator` component (`server-indicator.tsx`) is fully implemented with server switching, mDNS discovery, and health probing via a Popover, but the sidebar footer renders a bare, non-interactive server icon instead of using this component.

The `WindowControls` component in `sidebar-layout.tsx` shows a new-thread button when the sidebar is collapsed, but lacks a corresponding new-chat button.

## Goals / Non-Goals

**Goals:**
- Load home-directory sessions from the server on app startup so chats persist across restarts
- Add a clickable archive/unlink button to project items in the sidebar on hover
- Add a "New Chat" button in the collapsed sidebar window controls area
- Add a chat-creation icon on hover for session items in the sidebar thread list
- Wire the `ServerIndicator` component into the sidebar footer for interactive server management

**Non-Goals:**
- Changing the OpenCode server's session storage model
- Adding chat-specific API endpoints on the server
- Cloud sync or multi-device session persistence
- Changing how projects are discovered or listed

## Decisions

### 1. Home directory session loading: `loadProjectSessions` on startup

**Decision**: Call `loadProjectSessions(homeDirectory)` from `SidebarLayout` when the home directory is resolved and the server is connected.

**Rationale**: The existing `loadProjectSessions` function already handles pagination, session status loading, and Jotai store hydration. Reusing it for the home directory requires no new server API or data model changes. Sessions created in chat mode use the home directory as their `directory` field, so the existing `listSessions` API call filtered by directory naturally returns them.

**Alternative considered**: Creating a dedicated chat-session endpoint. Rejected as unnecessary — the existing session API works for any directory.

### 2. Archive button placement: hover-reveal icon next to expand arrow

**Decision**: In `ProjectFolder`, replace the existing `MessageCirclePlusIcon` hover button (which navigates to the project page — confusing UX) with an `ArchiveIcon` (or `FolderXIcon`) button that calls `onArchive(e, project)` to hide the project from the sidebar. Keep the expand/chevron as the primary interaction.

**Rationale**: Archive is a destructive-cosmetic action (hides from view, doesn't delete data). Hover-reveal keeps the UI clean while making the action discoverable. The existing `archiveProject()` function and `getArchivedProjectIds()` filtering is already wired up in `sidebar.tsx` — only the UI trigger is missing.

**Alternative considered**: Context menu (right-click). Rejected because archive is a frequent enough action to warrant a single-click affordance, and a hover button is already established by the SessionItem delete button pattern.

### 3. New Chat button in WindowControls

**Decision**: Add a `MessageCircleIcon` button next to the existing `MessageCirclePlusIcon` in `WindowControls`, visible only when the sidebar is collapsed. Clicking it calls `handleNavigateChat` (same as the sidebar Chat button).

**Rationale**: The collapsed-sidebar toolbar already has a new-thread button — adding a new-chat button next to it follows the same pattern and gives users one-click access to chat without opening the sidebar. The icon choice matches the existing "New Chat" sidebar button.

### 4. Chat icon on session hover

**Decision**: In `SessionItem`, add a `MessageCircleIcon` hover button (alongside the existing `TrashIcon` delete button) that navigates to `/` with `chatInitDirectory` set to the session's project directory.

**Rationale**: Users frequently want to start a chat in the context of a project they're already working in. The hover button pattern is already established by the delete button. Setting `chatInitDirectory` pre-selects the project directory in the NewChat view, streamlining the flow.

### 5. ServerIndicator in sidebar footer

**Decision**: Replace the bare server icon + status dot rendered in `SidebarFooter` with the existing `ServerIndicator` component. Remove the duplicate inline server icon render.

**Rationale**: The `ServerIndicator` component already has all the desired functionality (Popover with server list, mDNS discovery, health status, "Manage Servers" link). Duplicating this logic in the sidebar is unnecessary. The component was likely written for this exact purpose but never wired up.

## Risks / Trade-offs

- **Home directory session loading may fetch extra data**: If the home directory contains non-chat sessions (e.g., from a project whose worktree is the home dir), those will also appear. → Mitigation: The sidebar already filters sessions by `chatSessionIds` (directory === homeDirectory), so non-chat sessions in the home dir would not appear under "Chats" but would still be in the store (negligible memory impact).

- **Archive is local-only**: Archived project IDs are stored in `localStorage` and are not synced to the server. If the user clears localStorage or uses a different machine, archived projects reappear. → Acceptable: this matches the current architecture (no server-side project management API).

- **Window controls crowding**: Adding a third button (sidebar toggle + new thread + new chat) in the collapsed sidebar toolbar. → Mitigation: The 3 buttons fit within the existing `WINDOW_CONTROLS_LEFT` padding on macOS (93px from left edge provides ~70px for buttons, each ~28px wide = 84px total).
