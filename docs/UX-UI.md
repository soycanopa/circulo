# Circulo — UX / UI Specification

| Campo | Valor |
| --- | --- |
| Producto | Circulo |
| Versión | 0.4 |
| Fecha | 16 de agosto de 2026 |
| Plataforma | macOS, GPUI nativo |
| Referencia visual | Waku (inspiración de layout y densidad, no copia literal) |
| Complementa | `docs/PRD.md`, `docs/FLOWS.md` |

Este documento define la superficie. No es un kit de Figma. Los componentes se implementarán en GPUI. Nada de esto se construye sin un change de OpenSpec y permiso.

---

## 1. Principios de experiencia

1. **Comprensible sin manual.** Una persona de marketing no debe aprender vocabulario de agentes.
2. **Estructurado, no ruidoso.** El stream se lee como un documento vivo, no como un log.
3. **Rápido al tacto.** Hover, click, tipear y scroll deben sentirse nativos.
4. **Estados honestos.** Pendiente, generando, listo, error: siempre visibles y en lenguaje humano.
5. **Pocas acciones, bien hechas.** Si una acción no ayuda a crear, no entra en la UI del MVP.
6. **Oscuro, enfocado, elegante.** Dark theme por defecto. Sin personalización profunda en el MVP.

Copy de UI: **inglés**, corto, concreto, sin jerga (`tool call` se muestra como “Action” o el nombre legible de la herramienta, no como `fn edit_file`). Toda cadena sale de un locale (`en` primero). Los nombres técnicos pueden existir en detalle expandido, no en el primer vistazo.

---

## 2. Anatomía de la ventana

**Decisión cerrada:** no hay title bar nativo. Traffic lights y el botón de colapsar sidebar viven en el Sidebar.

```
┌────────────────────────────────────────────────────────────────────┐
│ ┌──────────────────┐                                               │
│ │ ● ● ●  [≡]       │  Session Header                               │
│ │                  │  título + acciones                            │
│ │  New session     ├───────────────────────────────────────────────┤
│ │  Search          │                                               │
│ │                  │           Messages Area                       │
│ │  Lista de        │           (scroll)                            │
│ │  sesiones        │                                               │
│ │                  │                                               │
│ │                  ├───────────────────────────────────────────────┤
│ │  Settings        │  Composer                                     │
│ └──────────────────┘                                               │
└────────────────────────────────────────────────────────────────────┘
```

### 2.1 Zonas

| Zona | Rol | Vacío / error |
| --- | --- | --- |
| Sidebar | Orientación: dónde estoy, qué más hay | Empty state de “crea un proyecto o una sesión” |
| Session Header | Contexto de la conversación actual | Placeholder si no hay sesión |
| Messages Area | El trabajo | Empty state de “escribe para empezar” |
| Composer | Crear el siguiente turno | Deshabilitado con razón clara si no hay sesión o el agente no está listo |

---

## 3. Sistema visual (MVP)

### 3.1 Dirección

- Dark theme único.
- Tipografía limpia, jerarquía clara (título de sesión > body markdown > meta > timestamps).
- Espaciado generoso pero no editorial-vacío.
- Radios y bordes suaves en cards (tool calls, diffs, tasks).
- Inline code y paths como pills/badges (inspiración de la referencia).
- Contraste suficiente para texto secundario; el gris no puede desaparecer.

### 3.2 Tokens (propuesta, a cerrar en el change de design system mínimo)

No inventar una paleta de 80 colores. Empezar con:

- Background app / sidebar / main (ligera diferencia de superficie).
- Border sutil.
- Texto primary / secondary / tertiary.
- Accent (un solo acento).
- Semánticos: success, warning, danger, info, streaming.
- Color de proyecto (opcional, viene del modelo).

Los valores hex se definen cuando se prototipe en GPUI, no se inventan aquí para “completar el doc”.

### 3.3 Iconografía

Heroicons. Estilo consistente (outline vs solid: **una** familia por contexto). No mezclar sets.

### 3.4 Movimiento

Solo animaciones nativas de GPUI, sutiles:

- Aparición de mensajes.
- Transición de estado de tool card (pending → running → success/error).
- Hover de `SessionItem`.
- Expand/collapse del Sidebar.

Prohibido: bounce decorativo, confetti, loaders teatrales, animar el markdown token a token si degrada lectura o performance.

### 3.5 Ventana

- Hidden / transparent title bar.
- Traffic lights en la esquina superior izquierda del Sidebar, con padding de macOS para que sean clicables.
- Botón de ocultar/mostrar **alineado** con los traffic lights (misma fila, `SidebarTopBar`).
- Sidebar colapsado: rail mínimo. Traffic lights + botón de expandir se quedan ahí. No migran al `SessionHeader`.

---

## 4. Componentes

Los nombres son del Project Definition. Mantenerlos.

### 4.1 `AppShell`

Layout sidebar + main. Conoce el estado colapsado. No conoce mensajes.

### 4.2 `Sidebar`

```
Sidebar
├── SidebarTopBar
│   ├── TrafficLights
│   └── CollapseSidebarButton
├── SidebarHeader
│   ├── NewSessionButton
│   └── SearchInput
├── TodaySection
│   └── SessionItem[]         (actividad en el día local)
├── EarlierSection
│   └── SessionItem[]         (actividad en días anteriores)
└── SidebarFooter
    └── SettingsButton
```

**Today:** sesiones cuya actividad (`last_message_at`, o `created_at` sin mensajes) cae en el día calendario local actual.

**Earlier:** sesiones con actividad en días anteriores (misma regla de timestamp).

Cada `SessionItem`:

1. **Nombre**
2. **Carpeta:** nombre del proyecto o **“Without Folder”**
3. **Duración** relativa a la derecha (`16m`, `14h`, `2d`)

**New session:** crea una sesión sin proyecto, la abre, y deja el selector del composer en **Without Folder**. No obliga a elegir carpeta.

**Search:** filtra títulos en Today y Earlier. Sin resultados → “No matching sessions”. No es un command palette.

Highlight si la sesión está seleccionada.

### 4.3 `SessionHeader`

- Título de la sesión (editable inline en P1).
- Acciones mínimas del MVP: no saturar. Candidatas (no todas se implementan): mover de proyecto, archivar. Pedir permiso por acción.
- Sin menús de developer (model picker avanzado, temperature, logs).

### 4.4 `MessagesArea`

Lista vertical scrolleable.

- Auto-scroll al fondo si el usuario está cerca del final durante el stream.
- Si el usuario scrollea hacia arriba, **no** secuestrar el scroll.
- Un control discreto tipo “saltar al final” si hay stream y el usuario no está abajo.

### 4.5 `Message`

Dos variantes visuales:

**UserMessage**

- Alineación y peso que distingan al usuario sin burbuja de iMessage caricaturesca.
- Contenido principalmente texto.

**AssistantMessage**

- `MessageMeta` opcional (p. ej. “Trabajó 10 segundos”) solo si el dato es real.
- Stack vertical de `MessagePart`.

No mostrar IDs internos.

### 4.6 `MessagePartRenderer`

Switch por tipo. Desconocido → fallback seguro (“No se puede mostrar este bloque”) , nunca crash.

### 4.7 `TextPart`

- Markdown de calidad: headings, listas, tablas, blockquote, code block, links, énfasis.
- Code block con overflow horizontal, no romper el layout.
- Inline code / paths en pill.
- Durante stream: el markdown incompleto no debe “parpadear” de forma agresiva. Preferir estabilidad visual.

### 4.8 `ToolCallCard`

Componente crítico de confianza.

Siempre visible en primer nivel:

- Nombre humano de la acción
- Estado: Pendiente / En curso / Listo / Error (indicador + color semántico)
- Una línea de contexto (archivo, query, etc.) si se puede extraer del input

Expandible:

- Input resumido
- Output: diff, texto, json colapsado, o error claro

Estados de movimiento: el card nace compacto y actualiza el indicador. No reemplazar el card entero de forma que salte el scroll.

**Diff:**

- Path del archivo arriba.
- +/- legibles, no un dump ANSI.
- Scroll interno si el diff es largo, para no explotar el mensaje.

### 4.9 `TaskList`

- Lista con estado visual (no necesariamente checkbox interactivo en el MVP).
- Título + estado.
- Orden según `order`.
- Interacción “marcar como hecha” es futura; no fingirla si no persiste.

### 4.10 `QuestionCard`

Fuera de implementación MVP. El renderer no debe crashear si llega una part `Question`: mostrar un fallback o esconder con log. Decisión de manejo: preguntar antes de implementar el fallback.

### 4.11 `Composer`

```
Composer
├── TextInput (multiline)
├── ComposerToolbar
│   ├── ProjectFolderSelector   (carpetas de proyecto + “Without Folder”)
│   ├── AgentSelector           (OpenCode, único en el MVP)
│   └── SendButton
└── ComposerFooter      (opcional; no en el primer corte)
```

**ProjectFolderSelector:** usable **solo al iniciar el chat** (antes del primer Send). Lista proyectos Active + **Without Folder**. Vacío = carpeta especial. Tras el primer envío: control **deshabilitado** (se ve la carpeta elegida, no se cambia). No hay cambio de worktree.

Reglas:

- Enter envía; Shift+Enter nueva línea. Convención a confirmar en el change de composer.
- Send deshabilitado si el texto está vacío o no hay sesión.
- Durante stream: indicador “Generating…” y, si PRD-CHT-09 se confirma, botón Cancelar.
- Placeholder y labels del selector salen de locale. Copy final se aprueba.

AgentSelector existe aunque solo haya un agente: educa el modelo mental para v0.3.

### 4.12 `StatusBar`

Opcional. No entra en el primer corte de UI.

### 4.13 Settings

Entrada en el footer del Sidebar. Contenido MVP:

- Estado de conexión con OpenCode.
- Sección **Archived projects**: lista + acción **Restore**. Al restaurar, el proyecto y sus sesiones vuelven al sidebar.

Borrar un proyecto (desde Settings u otra superficie de proyecto) pide confirmación: se van el proyecto y todas sus sesiones.

---

## 5. Empty, loading, error

| Situación | UI |
| --- | --- |
| Primera apertura, sin datos | Sidebar vacío + CTA “New session” |
| Sesiones sin carpeta | **Without Folder** en la card y en el composer |
| Proyecto archivado | Fuera de ambas vistas; visible en Settings → Archived projects |
| Sesión sin mensajes | Área central en calma + composer enfocado |
| OpenCode no encontrado | Banner no técnico (locale); sin stack trace |
| Fallo de red local / daemon | “Couldn’t connect to Circulo” (locale) + Retry |
| Mensaje con error | El mensaje se marca error; el resto del historial permanece |
| Búsqueda sin resultados | “No matching sessions” (locale) |

Nunca un panel en blanco sin siguiente paso.

---

## 6. Accesibilidad mínima (MVP)

GPUI no es la web. Aun así:

- Hit targets de traffic lights y botones primarios suficientemente grandes.
- Contraste de texto secundario revisado en dark.
- Focus visible en composer y search.
- No transmitir información solo con color (el estado de tool card lleva texto o icono + color).

No se promete VoiceOver completo en el MVP sin investigación.

---

## 7. Microcopy (dirección, no final)

| Contexto | Evitar | Preferir |
| --- | --- | --- |
| Tool running | `tool_call:running` | “Writing file…” / “Searching…” if known; else “Working…” |
| Diff | `hunk @ @@` as title | File path |
| Error adapter | `ECONNREFUSED 127.0.0.1` | “OpenCode isn’t available. Is it installed?” |
| Streaming | endless spinner | “Generating…” + Cancel if in scope |
| Unassigned session | “null” / empty | “Without Folder” |

Copy final se aprueba por pantalla, no se inventa en código.

---

## 8. Mapeo datos → UI

| Modelo | Componente |
| --- | --- |
| `Project` | línea de carpeta en `SessionItem` (o “Without Folder”) |
| `Session` | `SessionItem`, `SessionHeader` |
| `Message` | `UserMessage` / `AssistantMessage` |
| `MessagePart::Text` | `TextPart` |
| `MessagePart::ToolCall` | `ToolCallCard` |
| `MessagePart::TaskList` | `TaskList` |
| `MessagePart::Question` | no MVP |
| `ToolCall` / `ToolOutput::Diff` | card + diff viewer |

---

## 9. Fuera de alcance visual (MVP)

- Light theme, theme editor, acentos custom del usuario.
- Command palette.
- Multi-panel / split chat.
- Visualización de imágenes/archivos como galería rica (el modelo lo prevé a futuro).
- Onboarding animado largo.
- Personalización de densidad.

---

## 10. Criterios de aceptación de UX

Un cambio de UI no está listo si solo “se ve parecido a la referencia”. Está listo si:

1. Un no técnico entiende la pantalla sin explicación.
2. Los estados (vacío, stream, error) existen y son honestos.
3. No hay jerga suelta en el primer nivel.
4. Scroll y teclado se sienten nativos.
5. Se verificó en ventana compacta y amplia (no solo en un tamaño de desarrollo).
6. Se hizo prueba manual del flujo tocado (ver AGENTS.md).

---

## 11. Decisiones abiertas de UX

1. ~~Enter vs Shift+Enter (propuesta: Enter envía).~~ **Cerrada**: Enter envía, Shift+Enter nueva línea (UX-UI §4.11).
2. ¿El título se edita inline o en un diálogo? — fuera del MVP. P1.
3. ~~¿Settings es panel, ventana, o popover?~~ **Cerrada**: panel dentro de la ventana principal, anclado al footer del Sidebar; secciones General / Projects / Archived / Models en orden fijo.
4. ~~Densidad de `SessionItem` (dos vs tres líneas: nombre / tiempo / proyecto).~~ **Cerrada**: tres elementos por fila — nombre, carpeta (o **Without Folder**) debajo a la izquierda, duración relativa a la derecha.

Cerradas: restore desde Settings; selector locked tras primer send; sidebar Today/Earlier; sin worktree.
