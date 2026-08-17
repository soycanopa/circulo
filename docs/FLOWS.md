# Circulo — Flow Document

| Campo | Valor |
| --- | --- |
| Producto | Circulo |
| Versión | 0.4 |
| Fecha | 16 de agosto de 2026 |
| Complementa | `docs/PRD.md`, `docs/UX-UI.md`, `docs/TRD.md` |

Flujos del MVP. Un flujo no implementado no se “completa” improvisando. Si un paso depende de una decisión abierta, el flujo se detiene ahí hasta que se decida.

---

## 1. Leyenda

- **Actor:** Usuario, App (GPUI), Daemon, Adapter, OpenCode.
- **Camino feliz:** lo que debe funcionar para declarar el MVP usable.
- **Alternativos / error:** obligatorios si el feliz existe.
- **Fuera de MVP:** no se construye.

---

## 2. Arranque

```
Usuario abre Circulo (circulo-app)
        │
        ▼
App spawnea o reutiliza circulo-daemon (proceso 2)
        │
        ▼
App pide GET /v1/health al daemon
        │
        ├─ OK + OpenCode disponible → Shell
        ├─ OK + OpenCode ausente    → Shell + banner de setup
        └─ Daemon no responde       → pantalla de error local + reintentar
```

**Criterio:** el usuario no abre una terminal para levantar el daemon de Circulo.

**Abierto:** si el daemon debe spawnar `opencode serve` o solo conectarse.

---

## 3. Primera vez (sin datos)

```
Shell vacío (carpeta especial Sessions)
   │
   └─ CTA “New session” → flujo Crear sesión (sin proyecto)
```

No hay que crear un proyecto para empezar. Empty state: **New session**, no “No items”.

---

## 4. Crear proyecto

1. Usuario dispara “Nuevo proyecto” (ubicación exacta del control: a confirmar en UI).
2. Ingresa nombre (obligatorio). Descripción y color opcionales (P1).
3. App `POST /v1/projects`.
4. Sidebar muestra el proyecto.
5. Siguiente paso sugerido: crear sesión.

**Errores:** nombre vacío; fallo de persistencia → toast/inline error humano.

---

## 5. Crear sesión

1. Usuario pulsa “New session”.
2. App `POST /v1/sessions` **sin** `project_id`, `agent = OpenCode`.
3. El daemon persiste la sesión con `project_id = NULL` (carpeta especial `Sessions`).
4. Se selecciona la sesión. Header muestra título placeholder. Composer recibe foco. El selector de carpeta queda en **Without Folder**.
5. El item aparece en **Today** con nombre, duración relativa y **Without Folder**.

**Título:** placeholder (“New session”). Generación automática: P1 y abierta.

## 5.1 Elegir carpeta en el composer (antes del chat)

1. En el composer, el usuario abre **ProjectFolderSelector**.
2. Elige una carpeta de proyecto activa, o deja **Without Folder**.
3. Si elige proyecto: `PATCH` de la sesión con `project_id`. La sesión pasa a ese grupo.
4. Si no elige: permanece en la carpeta especial.
5. Recién entonces envía el mensaje (flujo 6).
6. Tras el primer send, el selector se bloquea. Un `PATCH` posterior de `project_id` es error.

No bloquear el envío si no hay proyecto: “sesión normal”.
No hay flujo de cambio de worktree.

---

## 6. Enviar mensaje y streaming (flujo núcleo)

```
Usuario escribe en Composer y envía
        │
        ▼
App crea Message user (optimistic o tras ACK)
        │
        ▼
POST /v1/sessions/{id}/messages
        │
        ▼
Daemon persiste el turno user
        │
        ▼
Daemon pide al Adapter generar
        │
        ▼
Adapter habla con OpenCode
        │
        ▼
Eventos SSE → Daemon normaliza → persiste incrementos → App reduce
        │
        ├─ text deltas     → TextPart crece
        ├─ tool events     → ToolCallCard aparece / actualiza
        ├─ task list       → TaskList aparece / actualiza
        ├─ completed       → Message.status = Complete; composer libre
        └─ failed          → Message.status = Error; copy humano; composer libre
```

### 6.1 Reglas de UI durante el stream

- Composer entra en estado “Generando…”.
- No enviar un segundo mensaje hasta complete/fail, **salvo** que se apruebe cola o cancelación.
- Auto-scroll solo si el usuario está anclado al final.
- Un tool call no debe resetear el scroll ni remountir todo el mensaje.

### 6.2 Cancelar (P1)

```
Usuario pulsa Cancelar
  → POST /v1/sessions/{id}/cancel
  → Adapter intenta abortar
  → Message queda Complete parcial o Error cancelado (decidir en el change)
  → Composer se libera
```

No implementar cancelación “fake” (solo parar el render y dejar el agente vivo).

---

## 7. Tool call visible

```
Evento tool started
  → Card compacta: nombre + “En curso”
Evento tool updated (input/output parcial)
  → Card actualiza sin saltar de layout de forma agresiva
Evento tool success + Diff
  → Card “Listo” + diff revisable
Evento tool error
  → Card “Error” + mensaje humano; resto del stream puede continuar
```

El usuario no necesita saber qué es un “adapter”. Sí necesita ver *qué se hizo* (archivo, búsqueda, etc.).

---

## 8. Navegar sesiones

```
Usuario click en SessionItem
  → SessionHeader y MessagesArea muestran esa sesión
  → GET messages si no están en memoria
  → Composer asociado a esa sesión
```

Si la sesión anterior estaba streameando, el stream **sigue en background** (el daemon no lo mata al cambiar de vista). Al volver, el estado está al día vía persistencia + eventos.

**Abierto:** ¿se puede streamear más de una sesión a la vez? Recomendación: permitir a nivel daemon, no prometer UI de múltiples streams visibles. Confirmar.

---

## 9. Sidebar Today y Earlier

```
Sidebar
  → sección Today: sesiones con actividad en el día local
  → sección Earlier: sesiones con actividad en días anteriores
  → cada item: nombre + carpeta | “Without Folder” + duración relativa (derecha)
  → búsqueda filtra ambas secciones
  → sección vacía no muestra header
```

## 9.1 Archivar proyecto

1. Acción Archive en el proyecto (Settings u otra superficie de proyecto).
2. `status = Archived`.
3. El proyecto y sus sesiones salen del sidebar.
4. Aparece en Settings → Archived projects.
5. Los datos **no** se borran.

## 9.1.1 Restaurar proyecto

1. Settings → Archived projects → Restore.
2. `POST /v1/projects/{id}/restore` → `status = Active`.
3. Sus sesiones reaparecen en Today o Earlier según actividad (con el nombre del proyecto en la card).

## 9.2 Borrar proyecto

1. Acción Delete + confirmación clara (se perderán N sesiones).
2. `DELETE /v1/projects/{id}`.
3. SQLite `ON DELETE CASCADE`: sesiones y mensajes de esa carpeta desaparecen.
4. Si la sesión abierta pertenecía a ese proyecto: volver a empty / New session.

No existe “borrar proyecto y conservar sesiones”. Eso sería desasignar, no borrar.

---

## 10. Buscar sesiones

1. Usuario escribe en Search del Sidebar.
2. Filtro por título (mínimo). Candidato P1: primeras líneas / nombre de proyecto.
3. Lista se reduce. Sin resultados → empty de búsqueda.
4. Limpiar search restaura la lista.

La búsqueda es local, sincrónica al dataset ya persistido. No es búsqueda semántica.

---

## 11. Colapsar Sidebar

```
Click hide
  → Sidebar se contrae a rail mínimo (animación nativa)
  → Traffic lights + botón show siguen alineados en el rail
  → Messages/Composer ganan ancho
Click show
  → Sidebar vuelve
```

La ventana nunca se queda sin traffic lights.

---

## 12. Settings (mínimo)

1. Footer → Settings.
2. Muestra estado: Circulo listo / OpenCode encontrado o no.
3. Acciones: reintentar detección. Nada más hasta que se apruebe.

---

## 13. Error: OpenCode no disponible

```
Health o envío detecta adapter unavailable
  → No se finge el stream
  → Banner o diálogo: OpenCode no está disponible
  → Mensaje del usuario: o no se envía, o queda Pending/Error (decidir; no perder texto del composer)
```

Preferencia de producto (a confirmar): el texto del composer no se borra nunca por un error de envío.

---

## 14. Archivar sesión (P1)

1. Acción Archivar.
2. Sale de la lista activa.
3. Recuperar no es crítico en el MVP; si no hay UI de archivadas, **no archivar** o hay que definir cómo se deshace.

Si no hay undo ni vista de archivo, no implementar archive.

---

## 15. Flujos fuera del MVP

- Preguntas interactivas (SingleSelect, Confirm, etc.).
- Cambiar de proveedor en una sesión existente.
- Compartir sesión / link / colaborar.
- Multi-agente en un mismo hilo.
- Sync entre máquinas.

---

## 16. Diagramas de secuencia (núcleo)

### 16.1 Turno de chat

```
Usuario → App: send(text)
App → Daemon: POST /messages
Daemon → Persist: save(user message)
Daemon → Adapter: generate(session, text)
Adapter → OpenCode: HTTP (API vigente)
OpenCode → Adapter: SSE/eventos
Adapter → Daemon: NormalizedEvent*
Daemon → Persist: upsert(message/parts)
Daemon → App: SSE session.*
App → UI: reduce + render
OpenCode → Adapter: done
Adapter → Daemon: Completed
Daemon → App: message.completed
```

### 16.2 Cambio de sesión durante stream

```
Daemon: stream sesión A sigue
Usuario → App: select sesión B
App → Daemon: GET /sessions/B/messages
App: render B
Daemon → App (sub A): eventos A (si la App sigue suscrita)
App: aplica a store de A, no pinta B con parts de A
```

El store de UI está indexado por `session_id`. Nunca mezclar parts.

---

## 17. Estados de sesión y mensaje

### SessionStatus

| Estado | Significado | UI |
| --- | --- | --- |
| Active | Visible, usable | Lista normal |
| Archived | Fuera de lista activa | Solo si hay UI de archivo |
| Error | Sesión inutilizable (p. ej. provider roto de forma persistente) | Indicador; no bloquear el resto |

Usar `Error` con parsimonia. Un turno fallido no convierte la sesión entera en Error.

### MessageStatus

| Estado | UI |
| --- | --- |
| Pending | Aún no aceptado / en cola |
| Streaming | Indicador en mensaje + composer ocupado |
| Complete | Estático |
| Error | Marca de error, historial intacto |

---

## 18. Criterios para dar un flujo por cerrado

1. Camino feliz recorrido a mano en macOS.
2. Al menos un error real recorrido a mano (daemon caído o OpenCode ausente, según el flujo).
3. No hay callejón sin empty/error state.
4. El comportamiento coincide con PRD + este documento. Si no, se actualiza el spec **antes** de “arreglar” a ojo.
5. Nada commiteado hasta esa prueba manual (AGENTS.md).
