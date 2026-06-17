## 1. Sidebar restructure (sidebar.tsx)

- [x] 1.1 Remove the `RECENT_COUNT` constant and the "Recent" section rendering block (lines 57, 163-170, 253-272)
- [x] 1.2 Remove the `onAddProject` plus button from the Projects/Threads header action row (lines 323-339)
- [x] 1.3 Remove `PlusIcon` import if no longer used
- [x] 1.4 Rename the Projects group label from "Projects" to "Threads" (line 280)
- [x] 1.5 Remove the `ChevronRightIcon` from `ProjectFolder`'s `SidebarMenuButton` — clicking the project itself expands (lines 517-520)
- [x] 1.6 Add `FolderIcon` and `FolderGit2` imports from `lucide-react`
- [x] 1.7 Display `FolderGit2` when project has a git repo (detect by hex commit hash `id` pattern), otherwise `FolderIcon`
- [x] 1.8 Add `ArchiveIcon` import and render it on project hover in Threads with `opacity-0 group-hover:opacity-100` pattern
- [x] 1.9 Implement archive click handler: add project ID to `localStorage` key `circulo-archived-projects`, filter from displayed projects
- [x] 1.10 Filter archived projects out of the `filteredProjects` list using the localStorage key
- [x] 1.11 Add `MessageCircleIcon` import and replace `StatusIcon` usage in `SessionItem` with `MessageCircleIcon`
- [x] 1.12 Keep status-based color and animation on the `MessageCircleIcon` (animate-spin when running)
- [x] 1.13 Update empty state text from "No projects yet" / "Add a project to get started" to "No threads yet"
- [x] 1.14 Remove import of unused status icons (`Loader2Icon`, `TimerIcon`, `CircleDotIcon`, `CheckCircle2Icon`, `AlertCircleIcon`) if no longer needed in sidebar.tsx
- [x] 1.15 Add "Chat" navigation item in the sidebar menu below the Threads section using `MessageCirclePlusIcon`
- [x] 1.16 Implement Chat navigation: clicking "Chat" navigates to Chat view or starts session in home directory

## 2. Chat sessions (threadless-chat)

- [x] 2.1 Determine the user's home directory (expose via `window.circulo` preload if not already available, or use existing `os.homedir()` IPC)
- [x] 2.2 Create logic to treat home directory as a special Threads project: add a synthetic "Chats" entry to the project list when chat sessions exist
- [x] 2.3 Filter agents whose `directory === homeDir` and group them under the Chats entry in Threads
- [x] 2.4 Wire the "Chat" sidebar item to navigate to the home-directory-scoped chat view
- [x] 2.5 Ensure Chat sessions persist and are discoverable across app restarts (OpenCode server sessions in home dir)

## 3. Sidebar toggle & window controls (sidebar-layout.tsx)

- [x] 3.1 Add `MessageCirclePlusIcon` import to `sidebar-layout.tsx`
- [x] 3.2 Subscribe to `useSidebar().open` state in `WindowControls`
- [x] 3.3 Conditionally render `MessageCirclePlusIcon` button only when `open` is false (sidebar collapsed)
- [x] 3.4 Remove the always-visible `PlusIcon` button from `WindowControls`
- [x] 3.5 Change the "New Thread" sidebar menu item icon in `AppSidebarContent` from `PlusIcon` to `MessageCirclePlusIcon`

## 4. Footer layout (server-indicator.tsx + sidebar.tsx)

- [x] 4.1 Change `SidebarFooter` from stacked layout to flex-row layout (`flex flex-row items-center p-2`)
- [x] 4.2 Render `ServerIndicator` and Settings button on the same row
- [x] 4.3 Right-align the server indicator or Settings with `ml-auto` / `mr-auto` (server indicator right, settings left or vice versa — server indicator aligned right per spec)

## 5. Verification

- [x] 5.1 Run `bun run lint` from root and fix any issues
- [x] 5.2 Run `cd apps/desktop && bun run check-types` and fix type errors
- [ ] 5.3 Manually verify: Threads section shows with folder icons, no caret, no Recent, no plus button
- [ ] 5.4 Manually verify: archive button appears on hover and removes project from list
- [ ] 5.5 Manually verify: Chat section appears and opens home-directory scoped chat
- [ ] 5.6 Manually verify: sidebar toggle shows new-thread button only when collapsed
- [ ] 5.7 Manually verify: server indicator sits beside Settings in footer
