## 1. Server indicator popover in sidebar footer

- [x] 1.1 In `sidebar.tsx`, replace the inline server icon + status dot in `SidebarFooter` with the existing `ServerIndicator` component from `server-indicator.tsx`
- [x] 1.2 Remove the inline `ServerIcon`, `isLocal`, `activeServer` derivation, and the bare `<SidebarMenu>` wrapper around the server icon — the `ServerIndicator` handles all of this internally

## 2. Project archive button on hover

- [x] 2.1 In `sidebar.tsx` `ProjectFolder`, replace the existing `MessageCirclePlusIcon` hover button (which incorrectly navigates to the project page) with an `ArchiveIcon` or `FolderXIcon` button that calls `onArchive(e, project)`
- [x] 2.2 Import `ArchiveIcon` (or `FolderXIcon`) from `lucide-react`
- [x] 2.3 Verify the hover button only renders when `onArchive` is provided and `isChatsFolder` is false

## 3. Chat persistence across app restarts

- [x] 3.1 In `sidebar-layout.tsx`, after the `homeDirectory` is resolved and the server is connected (`serverConnected` becomes true), call `loadProjectSessions(homeDirectory)` to fetch home-directory sessions from the server
- [x] 3.2 Import `loadProjectSessions` from `services/connection-manager.ts` into `sidebar-layout.tsx`
- [x] 3.3 Guard against duplicate loads: use a ref or a boolean state to ensure `loadProjectSessions(homeDirectory)` is called only once per home directory resolution

## 4. New Chat button in collapsed sidebar controls

- [x] 4.1 In `sidebar-layout.tsx` `WindowControls`, add a `MessageCircleIcon` button next to the existing `MessageCirclePlusIcon`, visible only when `!open` (sidebar collapsed)
- [x] 4.2 Wire the button's `onClick` to `handleNavigateChat` (same callback used by the sidebar Chat button)
- [x] 4.3 Import `handleNavigateChat` or accept it as a prop — refactor `WindowControls` to accept callbacks or use inline navigation with `chatInitDirectoryAtom`
- [x] 4.4 Ensure the tooltip reads \"New Chat\"

## 5. Chat icon on session hover in thread list

- [x] 5.1 In `sidebar.tsx` `SessionItem`, add a `MessageCircleIcon` hover button alongside the existing `TrashIcon` delete button that appears on `group-hover/session`
- [x] 5.2 Wire the button's `onClick` to set `chatInitDirectoryAtom` to the agent's `projectDirectory` and navigate to `/`
- [x] 5.3 Ensure the button has `aria-hidden` on the icon and a `sr-only` label for accessibility
