## Why

Actualmente Circulo solo muestra como slash-commands los comandos definidos en `.opencode/commands/` y las skills con `slash: true` registradas por el servidor. Las skills instaladas localmente en `.opencode/skills/` (como openspec-*, paseo-*, etc.) y las globales en `~/.agents/skills/` no aparecen al escribir `/` en el chat, obligando al usuario a abrir el skill-picker dialog para usarlas. Esto rompe el flujo natural de trabajo en el chat y crea inconsistencia con el TUI de OpenCode.

## What Changes

- El `SlashCommandPopover` obtiene skills desde `client.app.skills()` y las fusiona con los comandos existentes
- Las skills locales del proyecto (`.opencode/skills/`) aparecen como slash-commands sin requerir `slash: true` en su SKILL.md
- Las skills globales (`~/.agents/skills/`, `~/.config/opencode/skills/`) tambien aparecen como slash-commands
- Se usa el campo `name` del frontmatter de cada SKILL.md como nombre del comando (ej: `openspec-propose` → `/openspec-propose`)
- Los comandos provenientes de skills se marcan con el badge `skill` ya existente
- El `SkillPickerDialog` sigue funcionando igual (exploracion con descripcion completa)
- El hook `useSkills` existente se reutiliza en el popover de slash commands

## Capabilities

### New Capabilities
- `local-skills-slash-commands`: Las skills locales y globales aparecen en el popover de slash-commands del chat, permitiendo invocarlas escribiendo `/` seguido del nombre de la skill

### Modified Capabilities
_(ninguna — no se modifican specs existentes)_

## Impact

- `slash-command-popover.tsx`: Nuevo hook `useSkills`, merge de skills con commands
- `use-opencode-data.ts`: Se reutiliza el hook `useSkills` existente
- `skill-picker-dialog.tsx`: Sin cambios
- `chat-view.tsx`: Sin cambios (el popover ya esta integrado)
- `new-chat.tsx`: Hereda el comportamiento al reutilizar `SlashCommandPopover`
