## 1. Restore New Thread button on project hover

- [x] 1.1 In `sidebar.tsx` `ProjectFolder`, add a `MessageCirclePlusIcon` hover button alongside the existing `ArchiveIcon` inside the `onArchive && !isChatsFolder` guard
- [x] 1.2 Wire the new-thread button to navigate to `/` with `chatInitDirectoryAtom` set to the project's directory (same pattern as the SessionItem chat button)
- [x] 1.3 Set `aria-hidden` on both icons and `sr-only` on labels

## 2. Add hover buttons to Chats folder

- [x] 2.1 In `sidebar.tsx` `ProjectFolder`, when `isChatsFolder=true`, render a `MessageCircleIcon` hover button that starts a new chat (same as sidebar "New Chat" button)
- [x] 2.2 Do NOT render an archive button for the Chats folder
- [x] 2.3 Set `aria-hidden` on the icon and `sr-only` label "New Chat"

## 3. Ensure Chats folder auto-hides when empty

- [x] 3.1 Verify the existing guard `chatSessionIds.size > 0 && homeDirectory` already hides the Chats folder when empty — no code change needed, just verify behavior
