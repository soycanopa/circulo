## Context

The sidebar currently has three session display sections: Active Now, Recent, and Projects. Sessions are always stored in project directories, making the Recent section redundant. The Projects section uses a caret/chevron to expand sessions and a plus button to add projects. Icons are status-based (Loader2, Timer, etc.) which are visually busy. The server indicator ("This Mac") sits alone above Settings in the footer.

The change simplifies the sidebar into two clear navigation concepts: **Threads** (project folders + their sessions) and **Chat** (free-floating conversations in home directory). Project archiving gives users a way to permanently forget projects.

## Goals / Non-Goals

**Goals:**
- Remove the Recent section entirely
- Rename Projects to Threads, with folder icons (local vs git) instead of chevrons
- Click project name to expand (remove caret)
- Add archive button on project hover
- Add "Chat" section for home-directory conversations
- Conditional new-thread button in window controls (only when sidebar collapsed)
- Move server indicator beside Settings in footer
- Replace session status icons with a unified chat bubble icon

**Non-Goals:**
- Changing how automations work or where they appear
- Modifying Active Now behavior or filtering logic
- Changing session detail view or chat view
- Modifying the server panel popover or server discovery
- Changing project search/filter behavior
- Adding undo for archiving

## Decisions

### Decision 1: Git detection for folder icons

**Choice**: Use the existing `hasGitRepo` boolean from the project discovery data, or fall back to checking if `project.id` looks like a git commit hash (pure hex, 40 chars).

The project list atom derives from `discoveryAtom.projects` which contains OpenCode project data. OpenCode projects discovered from git repos have hex commit hash IDs. Local-only projects have a `dir-{hash}` format ID. We can detect this pattern.

**Alternatives considered**:
- Check for `.git` directory via filesystem read → too slow for sidebar rendering
- Add a `gitRepo` field to `SidebarProject` type → clean but requires type changes; the pattern detection is simpler

### Decision 2: Archive storage

**Choice**: Store archived project IDs in a `localStorage` key (`circulo-archived-projects`) as a JSON array of project IDs. Filter projects in the `useProjectList` hook or in the derived atom.

Archiving removes the project from the sidebar only; the actual files and sessions remain on disk. The user can re-add the project directory to restore it.

**Alternatives considered**:
- Jotai atom with localStorage persistence → adds complexity for a simple list
- Server-side archiving → requires API changes, overengineered for a sidebar feature
- Deletion of project files → dangerous, user may want files to remain

### Decision 3: Chat directory

**Choice**: Use `os.homedir()` (via Node.js `os` module, exposed through preload) to get the user's home directory. Start the OpenCode server in that directory for Chat sessions.

Chat sessions will be stored in `~/.local/share/opencode/projects/<home-dir-hash>/sessions/` and discovered alongside regular project sessions. The sidebar identifies Chat sessions by checking if `agent.directory === homeDir`.

**Alternatives considered**:
- Dedicated `~/Chats` directory → clean separation but adds yet another directory
- In-memory only chat → loses conversations on restart, contradicts requirement

### Decision 4: Chats group in Threads

**Choice**: Add a virtual "Chats" entry in the Threads project list. This is not a real OpenCode project but a synthetic sidebar item that groups all sessions whose directory equals the home directory.

The home directory is treated as a special project that always appears in Threads (un-archivable, or archivable separately). Sessions within display just like any project's sessions.

**Alternatives considered**:
- Separate Chat section entirely → would duplicate session list logic
- Add home dir as a real project → mixing real projects and synthetic ones is confusing

### Decision 5: Conditional window controls button

**Choice**: Use the `useSidebar()` hook's `open` state from `SidebarProvider` to conditionally render the `MessageCirclePlusIcon` button. The `PanelLeftIcon` toggle button always renders.

```tsx
const { open, toggleSidebar } = useSidebar()
// ...
<Button onClick={toggleSidebar}><PanelLeftIcon /></Button>
{!open && <Button onClick={newThread}><MessageCirclePlusIcon /></Button>}
```

**Alternatives considered**:
- CSS-only visibility toggle → less explicit, would still render DOM
- Separate collapsed/expanded render branches → more code, same result

### Decision 6: Footer layout change

**Choice**: Replace the stacked `SidebarFooter` layout with a flex row. The server indicator and Settings button sit side by side, with server indicator right-aligned via `ml-auto`.

```tsx
<SidebarFooter className="flex flex-row items-center p-2">
  <ServerIndicator compact />
  <SettingsButton className="ml-auto" />
</SidebarFooter>
```

The server indicator will need to be simplified when in the footer (shorter server name, smaller status dot, no wrap to multiple lines).

**Alternatives considered**:
- Keep server indicator on its own line, settings on another → doesn't satisfy requirement
- Move server indicator to header → would conflict with window controls

## Risks / Trade-offs

- **Risk**: Archive action is permanent and can't be undone → Mitigation: The project files and sessions are not deleted from disk, only hidden from sidebar. User can re-add via "Add Project" or manual directory entry.
- **Risk**: Home directory may contain many files unrelated to Circulo → Mitigation: OpenCode server only tracks Circulo sessions, not all files. The server is scoped to sessions it creates.
- **Risk**: `FolderGitIcon` may not exist in lucide-react → Mitigation: Use `FolderGit2` which is available in lucide-react 0.400+. Verify icon availability before implementing.
- **Risk**: Archive state lost on storage clear → Mitigation: Acceptable — it's a UI convenience, not critical data.

## Migration Plan

1. Deploy all sidebar changes together (they're cosmetic/UI-only)
2. No data migration needed — existing projects and sessions are unaffected
3. No backward compatibility concerns — this is a UI restructuring, not an API change
4. If rolled back, sidebar returns to previous state with no data loss

## Open Questions

- Should the Chats group be archivable like regular projects? (Assume yes, but remove the archive button if we decide it's special)
- Should the "New Thread" sidebar item open a project-scoped thread or a Chat? (Current behavior: navigates to `/` which is the new-thread page — keep this)
