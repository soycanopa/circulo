## Why

OpenCode slash commands (`/init`, `/review`, `/feedback`, `/mcp`, `/compact`, etc.) and skills are only available after a session starts — the new-chat screen has no slash command popover, no skills picker, and no `/` command execution. Additionally, `@file` mentions are sent to the server as raw text markers (e.g., `@index.ts`) instead of structured file references, making file resolution ambiguous. Skills with `slash: true` don't appear in the slash command popover.

## What Changes

- Add `SlashCommandPopover`, `SkillPickerDialog`, and slash command execution to the **new-chat screen**, matching the in-session behavior
- Convert `@file` mentions to structured file references in the prompt parts array (send resolved file path alongside the mention marker)
- Include skills with `slash: true` in the slash command popover (currently filtered out by `source !== "skill"`)
- Pass `agent` and `model` fields from SDK commands to `session.command()` when executing server commands
- Fix the local `Skill` type to include `slash` and `content` fields from the SDK's `SkillV2Info` type

## Capabilities

### New Capabilities
- `new-chat-slash-commands`: The new-chat screen supports slash commands and skills via the same `SlashCommandPopover` and `SkillPickerDialog` components used in the in-session chat view
- `structured-file-mentions`: When a user `@mentions` a file, the resolved file path is included as structured data in the prompt parts sent to the server
- `skill-slash-commands`: Skills with `slash: true` from the SDK appear as slash commands in the popover with their source indicated as "skill"

### Modified Capabilities
<!-- No existing capability requirements are changing -->

## Impact

- **Affected components**: `apps/desktop/src/renderer/components/new-chat.tsx` (add SlashCommandPopover, SkillPickerDialog, command execution), `apps/desktop/src/renderer/components/chat/chat-view.tsx` (extract reusable command execution, fix @mention→parts conversion), `apps/desktop/src/renderer/components/chat/slash-command-popover.tsx` (include skills with slash:true, pass agent/model), `apps/desktop/src/renderer/components/chat/skill-picker-dialog.tsx` (fix Skill type), `apps/desktop/src/renderer/hooks/use-server.ts` (structured file parts from mentions)
- **No API changes**: OpenCode SDK already supports structured file parts and `session.command()` with agent/model
- **No backend changes**: All changes are in the renderer
- **No breaking changes**: Existing in-session slash command and mention behavior is preserved
