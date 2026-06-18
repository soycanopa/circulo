## Context

Circulo's chat has two entry points: the **new-chat** screen (`new-chat.tsx`) and the **in-session** view (`chat-view.tsx`). Slash commands, skills, and @mentions are fully implemented in the in-session view but partially missing or broken in the new-chat screen and in how data is sent to the server.

**Current state:**
- **In-session (`chat-view.tsx`)**: Has `SlashCommandPopover`, `SkillPickerDialog`, `handleSlashCommand`, @mentions with file search. Commands execute via `client.session.command()`.
- **New-chat (`new-chat.tsx`)**: Has `MentionPopover` for @mentions but no slash command popover, no skills dialog, no slash command handling. Direct text messages only.
- **Slash popover** (`slash-command-popover.tsx`): Lists client commands + server commands but filters out skills (`c.source !== "skill"`). Does not pass `agent`/`model` from selected commands.
- **Mention sending** (`use-server.ts`): @mentions are sent as raw text markers in the `text` part. No structured file references.
- **Skill type** (`skill-picker-dialog.tsx`): Local `Skill` interface missing `slash` and `content` from SDK's `SkillV2Info`.

## Goals / Non-Goals

**Goals:**
- Slash commands and skills work on the new-chat screen (same experience as in-session)
- @file mentions include resolved file paths as structured data when sending to the server
- Skills with `slash: true` appear in the slash command popover
- Command execution passes `agent`/`model` overrides from SDK command definitions
- The local `Skill` type matches the SDK's `SkillV2Info`

**Non-Goals:**
- No changes to the `Cmd+K` command palette (`command-palette.tsx`)
- No new OpenCode SDK endpoints or backend routes
- No custom command icons beyond the existing 4 hard-coded ones
- No template preview for commands (the SDK `template` field remains unused)
- No command argument support from the popover (immediate execution on selection remains)

## Decisions

### 1. Extract reusable slash command execution to a shared hook

**Rationale**: Both `new-chat.tsx` and `chat-view.tsx` need the same slash command logic (detect `/` prefix, parse command name, dispatch to server/client handlers). Currently this logic lives inline in `ChatInputSection.handleSlashCommand` (chat-view.tsx lines 955-1012). Extract it to a shared hook (`use-slash-command.ts`) that both views can use.

**Alternative considered**: Duplicating the logic in new-chat. Rejected — violates DRY and risks divergent behavior.

**Details**: The hook exports:
- `executeSlashCommand(text: string, options): Promise<boolean>` — parses `/cmdName args` and dispatches
- Returns `false` if text doesn't start with `/`, `true` if command was handled

New-chat needs additional handling: for `/fork`, navigate to new session. For regular commands/skills, create a session first then execute the command. The hook accepts a `onFork` callback for this.

### 2. Add SlashCommandPopover and SkillPickerDialog to new-chat

**Rationale**: The new-chat screen should provide the same command/skill discovery as the in-session view. Components are already built and reusable.

**Implementation**: Add `SlashCommandPopover`, `TriggerDetector` (for `/` pattern), `SlashCommandBridge`, and `SkillPickerDialog` to `new-chat.tsx` following the same pattern as `chat-view.tsx` lines 1351-1374. Add `handleSlashCommand` using the shared hook.

### 3. Convert @file mentions to structured parts

**Rationale**: When a user types `@src/utils.ts` and sends the prompt, the OpenCode server must parse the text to find the file. With ambiguous filenames (multiple `index.ts` files), resolution is unreliable. The SDK supports structured `FilePartInput` with a resolved path.

**Implementation**: In `use-server.ts` `sendPrompt`, after building the `parts` array from text + attachments, also scan the `mentions` state for file mentions. For each `PromptMention` with `type: "file"`, include the resolved path in the parts array. The mention display name (`@filename`) stays in the text for readability.

**Alternative considered**: Replacing the text marker with the full path. Rejected — the `@filename` marker in text is useful context for the LLM; the structured part provides machine-readable resolution.

**Note**: The `PromptMention` type (defined in `prompt-mentions.ts`) already has an `option` field containing the `MentionOption` which for files includes the full path. This data just isn't forwarded to `sendPrompt`.

### 4. Include skills with `slash: true` in slash command popover

**Rationale**: OpenCode skills with `slash: true` are intended to be invokable via `/skillName`. The current filter `c.source !== "skill"` at slash-command-popover.tsx line 154 hides them. Skills should appear in the popover, grouped separately or marked with a skill badge.

**Implementation**: Remove the `c.source !== "skill"` filter, but only include skills where `slash` is `true`. Fetch skills alongside commands in the popover's data source. Add a visual indicator (badge or icon) for skill-sourced commands.

### 5. Pass agent/model overrides on command execution

**Rationale**: SDK commands can specify a default `agent` and `model`. When a user selects a command from the popover, these overrides should be forwarded to `session.command()`.

**Implementation**: Extend `handleSlashCommand` (in the shared hook) to accept optional `agent` and `model` parameters. When a command is selected from the popover, pass its `agent`/`model` fields. Update `session.command()` call to include them.

### 6. Fix Skill type to match SDK SkillV2Info

**Rationale**: The local `Skill` interface in `skill-picker-dialog.tsx` is missing `slash: boolean` and `content: string` from the SDK's `SkillV2Info`. This prevents proper skill handling.

**Implementation**: Add `slash?: boolean` and `content?: string` to the local `Skill` interface. Use `slash` to optionally distinguish skills in the UI.

## Risks / Trade-offs

- **[Risk] New-chat slash commands require session creation**: Some commands (like `/compact`) need an existing session. → **Mitigation**: For new-chat, most server commands will create a session implicitly. Client-only commands (undo/redo/compact) that require a session are skipped on new-chat with a toast notification.
- **[Risk] Skills in popover increase noise**: Adding skills alongside commands could make the popover crowded. → **Mitigation**: Only skills with `slash: true` are included; they're visually distinguished from built-in commands.
- **[Trade-off] @mentions still include text markers**: The `@filename` text remains in the prompt even with structured parts. Both are needed — text for LLM context, structured for machine resolution. → **Acceptable**: This matches how other AI tools (Cursor, Claude Code) handle file mentions.

## Open Questions

- Should the slash command popover show command descriptions from the SDK's `description` field? Currently only the command name is shown in the popover. → **Decision deferred**: Out of scope for this change, but the data is available if wanted later.
