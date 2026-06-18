## 1. Skill type and popover fixes

- [x] 1.1 Fix `Skill` interface in `skill-picker-dialog.tsx` to include `slash?: boolean` and `content?: string` fields
- [x] 1.2 Update `slash-command-popover.tsx` to include skills with `slash: true` in the command list (remove exclusion filter, add skill-only filter)
- [x] 1.3 Add visual indicator (badge or icon) for skill-sourced commands in the slash popover
- [x] 1.4 Pass `agent` and `model` from selected command to `handleSlashCommand` in `chat-view.tsx`
- [x] 1.5 Update `client.session.command()` call signature in `chat-view.tsx` to include `agent` and `model` parameters

## 2. Shared slash command hook

- [x] 2.1 Create `apps/desktop/src/renderer/hooks/use-slash-command.ts` — extract command parsing and execution logic from `chat-view.tsx` `handleSlashCommand`
- [x] 2.2 Hook exports `executeSlashCommand(text, options)` returning `Promise<boolean>`, with `options` accepting client, sessionID, callbacks (onFork, onSkillsOpen, onUndo, onRedo)
- [x] 2.3 Update `chat-view.tsx` to use the shared hook instead of inline `handleSlashCommand`
- [x] 2.4 Handle `onFork` for new-chat: navigate without executing via `client.session.revert()`

## 3. Add slash commands to new-chat screen

- [x] 3.1 Add `TriggerDetector`, `SlashCommandBridge`, `SlashCommandPopover` imports and rendering to `new-chat.tsx`
- [x] 3.2 Add `handleSlashCommand` via shared hook — create session then execute command for server commands, skip client-only commands (undo/redo/compact) with notification
- [x] 3.3 Add `SkillPickerDialog` rendering and `/skills` command handling to `new-chat.tsx`
- [x] 3.4 Wire `handleSend` in `new-chat.tsx` to detect `/` prefix and route to slash command handler

## 4. Structured file mentions

- [x] 4.1 Update `handleSend` in `chat-view.tsx` to pass `mentions` state alongside text and files
- [x] 4.2 Update `sendPrompt` in `use-server.ts` to accept `mentions` parameter and include structured file references in parts array
- [x] 4.3 Update `handleLaunch` in `new-chat.tsx` to pass `mentions` state to `sendPrompt`
- [x] 4.4 Ensure backward compatibility: prompts without mentions still produce text-only parts

## 5. Verification

- [x] 5.1 Run `bun run check-types` from root to verify no type errors
- [x] 5.2 Run `bun run lint` from root to verify no lint violations
