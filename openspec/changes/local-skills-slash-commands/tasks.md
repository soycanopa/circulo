## 1. Extract shared useSkills hook

- [x] 1.1 Add `skills` key to `queryKeys` in `use-opencode-data.ts`
- [x] 1.2 Extract `useSkills()` hook from `skill-picker-dialog.tsx` to `use-opencode-data.ts`, exporting it as a shared hook with proper types (using `SkillV2Info` from SDK)
- [x] 1.3 Refactor `skill-picker-dialog.tsx` to import and use the exported `useSkills` hook instead of the local function

## 2. Add skills to slash command popover

- [x] 2.1 In `slash-command-popover.tsx`, import `useSkills` from `use-opencode-data.ts` and add it alongside `useServerCommands`
- [x] 2.2 Map skills to `SlashCommand` format with `source: "server"`, `serverSource: "skill"`, icon `BookOpenIcon`, and no `agent`/`model`
- [x] 2.3 Merge skills into the `allCommands` array before server commands (order: skills, server commands, client commands)

## 3. Deduplicate skills and commands

- [x] 3.1 Filter out skills whose `name` already exists in the server commands array (commands take priority for agent/model info)
- [x] 3.2 Verify no duplicate entries appear when both a skill and command share the same name

## 4. Verify end-to-end flow

- [x] 4.1 Confirm skills selected from the popover are sent to `client.session.command()` via the existing `handleSlashCommand` flow
- [x] 4.2 Confirm the existing `skill` badge renders for skill-sourced entries in the popover
- [x] 4.3 Confirm `SkillPickerDialog` still opens correctly via `/skills` and functions unchanged
- [ ] 4.4 Manual test: type `/openspec` in chat input, verify openspec skills appear in popover with `skill` badge
- [x] 4.5 Run `bun run lint` and `bun run check-types` to verify no regressions
