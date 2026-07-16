# Changelog

Todos los cambios notables de Circulo se documentan aquí.

El formato sigue [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/), y el proyecto adhiere a [Semantic Versioning](https://semver.org/lang/es/).

## [Unreleased]

### Añadido

#### Composer (`/` y `@`)
- Pickers de comandos con patrón **shadcn Command** (`cmdk`): `src/components/ui/command.tsx`, `composer-command-menu.tsx`, `slash-command-picker.tsx`, `file-mention-picker.tsx`.
- Integración en `chat-input.tsx` con navegación por teclado (↑↓, Enter, Esc).
- Picker de slash commands y skills habilitados en el composer.
- Servidores MCP habilitados incluidos en el picker `/`.

#### Terminal integrada
- Panel PTY debajo del composer con **xterm.js** y `tauri-plugin-pty` (`terminal-panel.tsx`, `terminal-tab-pane.tsx`, `use-terminal.ts`).
- Toggle en el app bar (junto al de diffs) y atajo **⌘J**.
- Pestañas múltiples, barra de título delgada y superficie sin borde negro alrededor de xterm.
- Animación de apertura/cierre con easing `terminalDrawer` (0.34s).

#### Panel de diffs (estilo Synara)
- Panel derecho a **altura completa**, fuera de la zona de chat, como hermano de la columna principal en `sidebar-layout.tsx`.
- Toggle permanente en el app bar (izquierda del icono de terminal) con badge **+X −Y** de líneas modificadas (`diff-toggle-button.tsx`, `diff-stat-label.tsx`, `session-diff-stats.ts`).
- Lista de archivos + visor Pierre (`diff-review-panel.tsx`, `diff-file-list.tsx`).
- Recopilación de diffs por sesión (`session-diffs.ts`) y auto-apertura al recibir un diff nuevo durante un turno activo (`use-diff-panel-auto-open.ts`).
- Panel redimensionable con ancho persistido (`right-panel-resize-handle.ts`, `RIGHT_PANEL_*` en `preferences.ts`).
- Atajo **⌘⇧D** para abrir/cerrar el panel.
- Tool cards abren el panel lateral en lugar del overlay cuando el preview es un diff.

### Cambiado

#### Panel de diffs — layout y chrome
- Refactor del shell para alinear headers con `APP_BAR_HEIGHT` (40px), borde izquierdo y superficie tipo Synara.
- Título de sesión y acciones del app bar unificados en una sola fila (`MainAppBarChrome`) con alineación vertical consistente.
- Iconos de diff y terminal anclados al borde derecho de la **columna de chat** (dentro de `sidebar-inset`), no al borde de la ventana, para que sigan el resize del panel sin desincronizarse.

#### Panel de diffs — animación
- Animación de entrada/salida alineada con la terminal: `width` + `opacity` con `terminalDrawer`, componente dedicado `diff-panel.tsx`.
- Eliminados spring/`layout` que producían cierre brusco; el grid interno permite colapsar a ancho 0 (`minmax(0, …)`).
- Un solo mensaje vacío centrado cuando no hay archivos modificados (antes había dos columnas con textos duplicados).

#### Proyecto Chats
- La carpeta **Chats** en el sidebar solo se muestra si hay sesiones visibles.
- Al eliminar la última sesión del proyecto Chats, se cierra el proyecto automáticamente.
- En el arranque, no se reabre un proyecto Chats vacío.

#### Sesiones
- El sidebar lista solo chats rastreados por Circulo por proyecto (no sesiones huérfanas del agente).

### Corregido

#### Composer
- Estructura y estilos del picker alineados con el CommandDemo de shadcn.

#### Terminal
- Borde negro alrededor del viewport de xterm.
- Barra de título más delgada y tabs funcionales.

#### Panel de diffs
- Toggle de diffs oculto cuando no había cambios (ahora siempre visible junto a terminal).
- Toggle movido de vuelta al app bar junto al de terminal (no en el título de sesión).
- Botón **X** del panel no respondía: `WindowDragStrip` (z-48) cubría el header; corregido con `z-[50]`/`z-[52]` y sin drag region en el header del panel.
- Iconos del app bar que no seguían el borde derecho al redimensionar el panel de diffs.

### Commits de referencia

```
ddbb813 fix(bootstrap): skip reopening an empty chats project on launch
0582ec1 fix(sessions): close chats project after deleting the last session
7d7758d fix(sidebar): hide Chats folder when it has no sessions
3687ccb Fix diff panel close button blocked by window drag strip
f62ead7 Fix diff panel drawer animation to mirror terminal panel
ec9ccdc Match diff panel animation to terminal drawer easing
808dc05 Smooth diff panel animation and fix app bar alignment
fb6a3d4 Anchor app bar actions to chat column during diff resize
1ef9d83 Use single empty state in diff panel when no files changed
511db46 Always show diff toggle beside terminal in app bar
145d925 Restore diff toggle next to terminal in app bar
acdd2e3 Refactor diff panel layout to match Synara chrome
01efbe2 feat(diff): panel derecho de cambios estilo Synara
090009f feat(terminal): animación suave al abrir y cerrar el drawer
f7548bb fix(terminal): title bar delgado, tabs y sin borde negro de xterm
88428a4 feat(terminal): panel PTY debajo del composer con toggle en app bar
55e7c16 fix(composer): replicar estructura y estilos del CommandDemo de shadcn
b8419c1 fix(composer): alinear pickers con el patrón visual shadcn Command
d4bf620 feat(composer): usar Command (cmdk) para pickers / y @
298084b fix(sessions): show only Circulo-tracked chats per project
440c080 feat(chat): include enabled MCP servers in slash picker
9da452c feat(chat): add slash command and skill picker in composer
```

## [0.1.0] — 2026-07-15

Versión base del cliente Tauri ACP: chat streaming, permisos, credenciales, tool calls, plan mode, overlays, sidebar de proyectos, settings overlay y diffs inline en el transcript.