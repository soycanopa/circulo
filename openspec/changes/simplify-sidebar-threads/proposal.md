## Why

The sidebar has grown cluttered with sections (Active Now, Recent, Projects) that create cognitive overhead. Sessions are always stored in their project directory, making the "Recent" section redundant. We need to simplify navigation into two clear concepts: **Threads** (projects with their sessions) and **Chat** (free-floating conversations without a project), while adding the ability to archive/forget projects.

## What Changes

- **Remove** the "Recent" sessions section entirely — sessions live in their project under Threads
- **Remove** the plus button for adding projects in the Projects header — project addition is handled elsewhere
- **Remove** the caret/chevron icon on project items — clicking the project name itself expands it
- **Rename** "Projects" section to "Threads" — contains project folders and their sessions
- **Replace** session status icons with a chat bubble icon (`MessageCircleIcon`) for session items
- **Replace** project item icons: `FolderIcon` for local projects, `FolderGitIcon` for git/GitHub repos
- **Add** archive capability: hovering a project in Threads shows an archive/forget button that removes the project and its conversations permanently
- **Add** "Chat" section after Threads — starts conversations in the user's home directory, with sessions grouped as "Chats" in Threads
- **Change** the sidebar toggle area's "New Thread" button to use a chat-bubble-plus icon (`MessageCirclePlusIcon`) and show it only when the sidebar is collapsed
- **Move** the server indicator ("This Mac") next to Settings in the sidebar footer, aligned to the right

## Capabilities

### New Capabilities

- `sidebar-threads`: Threads section replacing Projects, with folder icons (local vs git), click-to-expand (no caret), and project archiving via hover action
- `threadless-chat`: Free-floating chat sessions scoped to the user's home/root directory, grouped under "Chats" in Threads
- `sidebar-toggle`: Updated window controls with conditional new-thread button (chat-bubble-plus icon, visible only when sidebar is collapsed)

### Modified Capabilities

<!-- No existing specs to modify -->

## Impact

- `apps/desktop/src/renderer/components/sidebar.tsx` — Major restructure: remove Recent section, rename Projects to Threads, change icons, add archive button, add Chat section
- `apps/desktop/src/renderer/components/sidebar-layout.tsx` — Update WindowControls to show new-thread button only when collapsed, change icon
- `apps/desktop/src/renderer/components/server-indicator.tsx` — Layout change to sit beside Settings in footer
- `apps/desktop/src/renderer/lib/types.ts` — Possibly add `isArchived` field to SidebarProject
- `apps/desktop/src/renderer/atoms/` — May need new atoms for archived projects tracking
- `apps/desktop/src/renderer/hooks/` — May need new hook for Chat directory discovery
