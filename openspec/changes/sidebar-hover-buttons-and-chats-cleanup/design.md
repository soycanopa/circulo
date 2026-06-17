## Context

The previous change `chat-persistence-and-ui-improvements` replaced the project hover button (which was a `MessageCirclePlusIcon` that incorrectly navigated to the project page) with an `ArchiveIcon` that archives the project. However, this removed the "New Thread" quick-access entirely. Users expect both actions on project hover: start a new thread and archive.

Additionally, the "Chats" synthetic folder and its child sessions have no hover buttons at all, and the Chats folder remains visible even when all its sessions are deleted.

Current state in `ProjectFolder`:
- Projects (`isChatsFolder=false`): Single `ArchiveIcon` hover button
- Chats folder (`isChatsFolder=true`): No hover buttons

## Goals / Non-Goals

**Goals:**
- Projects: Show BOTH a new-thread button (`MessageCirclePlusIcon`) and an archive button (`ArchiveIcon`) on hover
- Chats folder: Show a new-chat button (`MessageCircleIcon`) on hover
- Chat sessions (inside Chats folder): Show delete button on hover
- Chats folder: Auto-hide from sidebar when `chatSessionIds` set is empty

**Non-Goals:**
- Adding hover buttons to session items inside regular projects (already handled by previous change)
- Changing the archive behavior or storage mechanism

## Decisions

### 1. Two hover buttons on projects: new-thread + archive

**Decision**: Render both icons inside the `onArchive && !isChatsFolder` guard, rearranging so `MessageCirclePlusIcon` starts a new thread (navigates to `/` with `chatInitDirectory` set) and `ArchiveIcon` archives the project.

**Rationale**: The original code had a `MessageCirclePlusIcon` that navigated to the project page — wrong behavior. The correct "new thread" action is starting a chat in that project's directory. This gives two clearly distinct actions with different icons.

**Alternative considered**: Context menu for archive. Rejected — two hover buttons are more discoverable and the pattern is already established.

### 2. Chats folder: new-chat button only (no archive)

**Decision**: When `isChatsFolder=true`, show only a `MessageCircleIcon` hover button that starts a new chat (same as the sidebar "New Chat" button). Do NOT render an archive button — the Chats folder is synthetic and represents the user's home directory, which cannot be archived.

**Rationale**: The Chats folder is a presentation-layer grouping, not a real project. Archiving it doesn't make sense. But a quick "new chat" action on hover is convenient.

### 3. Chats folder auto-hide when empty

**Decision**: The Chats folder rendering is already guarded by `chatSessionIds.size > 0 && homeDirectory`. If `chatSessionIds.size` becomes 0 (all chat sessions deleted), the folder naturally disappears. No additional logic needed — this is already the current behavior, but was overlooked because the delete button on chat sessions was missing.

**Rationale**: The guard `chatSessionIds.size > 0` already exists. What was missing was the ability to delete chat sessions (no hover delete button on chat session items), so the set never shrank to 0. Adding the delete button completes the loop.

## Risks / Trade-offs

- **Crowded hover bar on narrow sidebar**: Two buttons (new-thread + archive) on project hover may feel cramped at narrow sidebar widths. → Mitigation: Buttons are only 20px each with tight spacing; they fit within the existing `group-hover` pattern already used by SessionItem.
