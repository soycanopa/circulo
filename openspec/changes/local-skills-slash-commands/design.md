## Context

Circulo obtiene comandos slash de dos fuentes: comandos definidos en `.opencode/commands/*.md` y skills con `slash: true` registrados por el servidor OpenCode. El `SlashCommandPopover` usa `useServerCommands()` que llama a `client.command.list()` (endpoint `GET /command?directory=...`). Este endpoint devuelve el tipo `CommandV2Info` del SDK.

Las skills que NO tienen `slash: true` no son devueltas por `command.list()`. Estan disponibles via `client.app.skills()` (tipo `SkillV2Info`), que actualmente solo se usa en `SkillPickerDialog`. Las skills tienen campos `name`, `description`, `location` ("global" o "project"), y `content`.

**Estado actual del codigo:**
- `slash-command-popover.tsx`: Solo consume `useServerCommands(directory)`. Ya tiene badge `skill` para `serverSource === "skill"`.
- `skill-picker-dialog.tsx`: Usa `useSkills(directory)` → `client.app.skills()`. Tiene la interfaz `Skill` local con `name`, `description`, `location`.
- `use-opencode-data.ts` (line ~419): `useServerCommands()` usa TanStack Query con key `["commands", directory]`. `useSkills()` existe en otro hook.
- `chat-view.tsx`: `handleSlashSelect` recibe `SlashCommand`, cierra popover y ejecuta via `handleSlashCommand`. Las skills se ejecutan igual que comandos (via `client.session.command()`).

## Goals / Non-Goals

**Goals:**
- Skills locales y globales aparecen en el `SlashCommandPopover` junto a los comandos existentes
- Al escribir `/` y buscar, las skills se filtran fuzzy junto con los comandos
- Seleccionar una skill del popover la ejecuta como slash-command (igual que cualquier comando)
- Las skills se marcan con el badge `skill` para distinguirlas visualmente
- El `SkillPickerDialog` sigue funcionando sin cambios

**Non-Goals:**
- No se modifica el formato SKILL.md de las skills (no se requiere añadir `slash: true`)
- No se añaden nuevos endpoints ni se modifica el servidor
- No se elimina el `SkillPickerDialog`
- No se cambia la ejecucion de slash-commands (`use-slash-command.ts`)

## Decisions

### 1. Añadir skills como source adicional en SlashCommandPopover

**Decision**: El popover obtiene skills via un nuevo hook `useSkillsForCommands(directory)` y las fusiona con `useServerCommands(directory)`.

**Razon**: `client.command.list()` solo devuelve skills con `slash: true`. Para incluir TODAS las skills, debemos consumir `client.app.skills()` independentemente. No queremos modificar las skills (añadir `slash: true`) porque son archivos externos gestionados por herramientas como openspec o speckit.

**Alternativa considerada**: Modificar los SKILL.md locales para añadir `slash: true`. Rechazada — las skills son regeneradas por herramientas externas y los cambios se perderian.

### 2. Mapear SkillV2Info → SlashCommand

**Decision**: Convertir cada `SkillV2Info` a un `SlashCommand` con:
- `name` = skill.name
- `description` = skill.description ?? `Run /${skill.name}`
- `source` = `"server"`, `serverSource` = `"skill"`
- `icon` = `BookOpenIcon` (mismo que `/skills`)
- Sin `agent` ni `model` (las skills delegan al agente por defecto)

**Razon**: Las skills no tienen `agent`/`model` en su tipo `SkillV2Info`. El servidor usa el agente configurado.

**Alternativa considerada**: Usar `getCommandIcon(skill.name)` para iconos variados. Rechazada — las skills no tienen iconos semanticos predefinidos; un icono uniforme es mas claro.

### 3. Fusion con deduplicacion

**Decision**: Las skills se fusionan ANTES de los comandos del servidor (orden: skills globales, skills locales, server commands, client commands). Si una skill tiene el mismo nombre que un comando existente, el comando tiene prioridad (ya tiene `agent`/`model` definidos).

**Razon**: Esto evita duplicados cuando una skill con `slash: true` ya esta registrada como comando. Las skills sin `slash: true` nunca colisionan porque el servidor no las expone como comandos.

### 4. Reutilizar useSkills existente sin modificar

**Decision**: Crear un wrapper o usar directamente el hook `useSkills` que ya existe en `use-opencode-data.ts`, devolviendo las skills sin filtrar.

**Razon**: `useSkills` ya esta implementado con TanStack Query y caching. No queremos duplicar la logica de fetching.

**Alternativa considerada**: Crear un nuevo hook `useSkillsForCommands`. Rechazada parcialmente — puede ser un simple wrapper sin logica nueva.

### 5. Ejecucion de skills seleccionadas del popover

**Decision**: Cuando el usuario selecciona una skill del popover, se ejecuta igual que un comando normal: se inserta `/<skillName>` en el textarea y se envia via `client.session.command()`.

**Razon**: El mecanismo de ejecucion actual (`use-slash-command.ts`) ya maneja comandos desconocidos enviandolos al servidor. Las skills se ejecutan correctamente de esta forma porque el servidor las reconoce por nombre. No se necesita logica especial.

## Risks / Trade-offs

- **[Risk] Skills sobrecargan el popover**: Añadir ~15 skills (8 openspec + 7 globales) a los comandos existentes puede hacer el popover largo. → **Mitigacion**: El filtro fuzzy (`fuzzysort`, threshold 0.3) reduce la lista efectiva. Las skills aparecen con badge `skill` para claridad visual.
- **[Risk] Colisiones de nombres**: Si una skill local y una global tienen el mismo nombre. → **Mitigacion**: La deduplicacion da prioridad al primer match encontrado (skills locales primero, luego globales).
- **[Trade-off] Skills sin descripcion util**: Muchas skills tienen descripciones largas generadas automaticamente. → **Aceptable**: Solo se muestran en el popover; el usuario puede usar `/skills` para ver detalles completos en el dialog.

## Open Questions

- ¿Deberian las skills globales filtrarse por proyecto? Actualmente `client.app.skills()` devuelve todas las skills (globales + locales) para el directorio dado. Mantenemos este comportamiento.
- ¿Icono unico por skill? Dejamos `BookOpenIcon` para todas. Se puede mejorar en el futuro con metadata de la skill.
