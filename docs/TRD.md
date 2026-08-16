# Circulo — Technical Requirements Document (TRD)

| Campo | Valor |
| --- | --- |
| Producto | Circulo |
| Versión | 0.2 |
| Fecha | 16 de agosto de 2026 |
| Estado | Pre-MVP / planificación |
| Complementa | `docs/PRD.md` |

Este documento define **cómo** se construye el sistema. No autoriza código. Cada incremento se implementa vía OpenSpec después de investigación y permiso explícito.

---

## 1. Resumen técnico

Circulo son **dos procesos propios**, más OpenCode como proceso externo:

1. **`circulo-app`** (GPUI) — UI. Solo habla el contrato de Circulo.
2. **`circulo-daemon`** — API local, persistencia SQLite, orquestación, adapters.
3. **OpenCode server** (`opencode serve`) — no es Circulo. El daemon lo alcanza vía el adapter.

El frontend **no** habla con OpenCode. Quien habla HTTP + SSE con OpenCode es el **daemon**, a través de `circulo-adapter-opencode`.

Hay **dos hop de red**, no uno:

```
┌──────────────────────────────────────────┐
│         circulo-app (proceso 1)          │
│   shell, chat, sidebar, renderers        │
└──────────────────┬───────────────────────┘
                   │ HTTPS + SSE
                   │ contrato Circulo (localhost)
                   ▼
┌──────────────────────────────────────────┐
│        circulo-daemon (proceso 2)        │
│   API Circulo, SQLite, orquestación      │
└──────────────────┬───────────────────────┘
                   │ HTTP + SSE
                   │ API de OpenCode (localhost)
                   ▼
┌──────────────────────────────────────────┐
│     OpenCode server (proceso externo)    │
│     `opencode serve`  (p. ej. :4096)     │
└──────────────────────────────────────────┘
```

Corrección frecuente: “Circulo habla HTTP/SSE con OpenCode” es verdad **solo para el daemon**. La app GPUI no conoce ese protocolo. OpenCode publica su propia OpenAPI + `GET /event` (SSE); el adapter traduce eso al modelo de Circulo.

---

## 2. Decisiones técnicas cerradas

| Tema | Decisión | Implicación |
| --- | --- | --- |
| UI | GPUI (Rust), componentes propios | No hay kit Shadcn listo; se implementan los componentes necesarios |
| Estilo | Sistema propio, inspiración Tailwind/Shadcn | Tokens propios, no CSS de web |
| Iconos | Heroicons | Hay que definir cómo se empaquetan (SVG → GPUI) |
| Animaciones | Nativas de GPUI | Prohibido Anime.js u otros runtimes JS de animación |
| Tooling de scripts | Bun + TypeScript | Solo scripts/tooling, no el runtime de la app |
| Daemon | Rust, deliberadamente simple | No es un orchestrator genérico en el MVP |
| Transporte app ↔ daemon | HTTPS + SSE (contrato Circulo) | Streaming de eventos tipados |
| Transporte daemon ↔ OpenCode | HTTP + SSE (API OpenCode) | Documentado en https://opencode.ai/docs/server/ |
| Procesos Circulo | Dos: `circulo-app` y `circulo-daemon` | OpenCode es un tercer proceso externo |
| Persistencia | SQLite | Un archivo local; migraciones versionadas |
| i18n | Locale files; default `en` | Cero strings de UI hardcodeados |
| Modularidad | Adapters como módulos independientes | Trait estable; OpenCode es el primer impl |
| Plataforma MVP | macOS | Window chrome custom (hidden title bar) |

---

## 3. Principios de arquitectura

1. **Modular por construcción.** Crates pequeños, fronteras claras, un adapter = un crate.
2. **El frontend es tonto respecto al proveedor.** Solo consume el modelo de Circulo.
3. **El daemon es simple.** Conversar bien, persistir, streamear. Nada más en el MVP.
4. **Contrato JSON estable** entre app y daemon. Serializable, versionable.
5. **Tipos compartidos** en un crate `core` / `protocol`. No duplicar structs entre capas.
6. **Falla de forma explícita.** Errores tipados, mapeables a copy humano.
7. **Testeable sin GUI.** Core, protocol y adapters se prueban sin GPUI.
8. **No asumir el protocolo de OpenCode.** Investigar la API real (`opencode serve`) antes de escribir el adapter.

---

## 4. Estructura de crates (propuesta, sujeta a permiso)

Workspace Cargo. Nombres tentativos:

| Crate | Responsabilidad | Depende de |
| --- | --- | --- |
| `circulo-core` | Entidades de dominio, IDs, errores de dominio | — |
| `circulo-protocol` | Request/response/SSE events, versionado de API | `circulo-core` |
| `circulo-adapter` | Trait `AgentAdapter` + tipos de stream del agente | `circulo-core` |
| `circulo-adapter-opencode` | Implementación OpenCode | `circulo-adapter` |
| `circulo-persist` | SQLite: projects, sessions (`project_id` nullable), messages | `circulo-core` |
| `circulo-daemon` | Binario: HTTP/SSE Circulo, orquestación | protocol, persist, adapter |
| `circulo-app` | Binario GPUI: ventana, UI, cliente del daemon | protocol |
| `circulo-i18n` | Carga de locales, lookup de claves, fallback `en` | — |
| `circulo-markdown` | Parseo y layout de Markdown para GPUI | — |

Reglas:

- `circulo-app` **no** depende de `circulo-adapter-opencode`.
- `circulo-adapter-opencode` **no** depende de GPUI.
- Un segundo adapter futuro es un crate nuevo + registro en el daemon. Nada más.

No crear el workspace hasta que exista un change de OpenSpec aprobado.

---

## 5. Modelo de datos

Fuente de verdad conceptual: sección 12 de `Circulo-Project-Definition.md`. Resumen normativo para ingeniería:

### 5.1 Grafo

```
Project 0..1 ──< Session 1 ──< Message 1 ──< MessagePart
```

La carpeta especial `Sessions` no es una fila `Project`. Son las sesiones con `project_id IS NULL`.

### 5.2 Entidades

- **Project:** `id` (UUID), `name`, `description?`, `color?`, `created_at`, `updated_at`. Solo proyectos que el usuario crea.
- **Session:** `id`, `project_id: Option<UUID>`, `title`, `agent` (`AgentType`), `status` (`Active` | `Archived` | `Error`), timestamps, `last_message_at?`.
- **Message:** `id`, `session_id`, `role` (`User` | `Assistant` | `System`), `parts`, `status` (`Pending` | `Streaming` | `Complete` | `Error`), `created_at`, `is_streaming`.
- **MessagePart:** unión discriminada:
  - `Text { content }` (Markdown)
  - `ToolCall { tool_call }`
  - `TaskList { tasks }`
  - `Question { question }` — **presente en el modelo, no implementada en UI del MVP**
- **ToolCall:** `id`, `name`, `status`, `input` (JSON), `output?`, timestamps.
- **ToolOutput:** `Text` | `Diff` | `Json` | `Error`.
- **Task:** `id`, `title`, `description?`, `status`, `order`.

### 5.3 Reglas

- IDs UUID, serialización JSON limpia (serde, `rename_all = "snake_case"` o acuerdo único documentado).
- Durante streaming, un `Message` puede ganar parts o actualizar parts existentes (sobre todo `ToolCall`).
- El frontend trata las actualizaciones como eventos aplicados a un reducer, no como “reemplazar todo el chat” en cada token si eso degrada el render. La estrategia exacta se decide en el design del change de streaming.

### 5.4 Persistencia (cerrada)

- Motor: **SQLite**.
- Ubicación candidata: `~/Library/Application Support/Circulo/circulo.sqlite` (confirmar path exacto en el change de persist).
- `sessions.project_id` es nullable (FK a `projects`, `ON DELETE SET NULL`).
- Migraciones versionadas en el crate `circulo-persist`.
- Búsqueda de sesiones: SQL sobre título (MVP).
- No hay una tabla “Inbox”. La carpeta especial es la query `WHERE project_id IS NULL`.

---

## 6. Contrato app ↔ daemon

Transporte: HTTPS en localhost + SSE para eventos.

### 6.1 Requisitos del transporte

| ID | Requisito |
| --- | --- |
| TRD-API-01 | El daemon escucha solo en localhost. |
| TRD-API-02 | HTTPS. El esquema de certificados locales es decisión abierta (self-signed de máquina, rustls, etc.). Investigar antes de implementar. |
| TRD-API-03 | El stream de una generación es SSE con eventos tipados, no un blob opaco. |
| TRD-API-04 | El contrato lleva `api_version`. Cambios breaking = version nueva o change de OpenSpec explícito. |
| TRD-API-05 | Errores de API tienen código estable + mensaje humano. |

### 6.2 Superficie mínima del MVP (borrador)

No es OpenAPI final. Es el perímetro. El schema concreto se especifica en el change correspondiente.

**Projects**

- `GET /v1/projects`
- `POST /v1/projects`
- `PATCH /v1/projects/{id}`
- `DELETE /v1/projects/{id}` — no borra sesiones; deja `project_id = null`

**Sessions**

- `GET /v1/projects/{id}/sessions`
- `GET /v1/sessions?q=` — lista plana; `project_id` opcional en el payload
- `GET /v1/sessions?unassigned=true` — carpeta especial `Sessions`
- `POST /v1/sessions` — crea sin proyecto salvo que se envíe `project_id`
- `GET /v1/sessions/{id}`
- `PATCH /v1/sessions/{id}` (título, `project_id`, archive)
- `GET /v1/sessions/{id}/messages`

**Chat**

- `POST /v1/sessions/{id}/messages` — crea el mensaje de usuario y arranca generación
- `GET /v1/sessions/{id}/events` — SSE: tokens, parts, tool calls, status, error
- `POST /v1/sessions/{id}/cancel` — P1

**Health**

- `GET /v1/health` — daemon + estado del adapter (OpenCode reachable / missing / error)

### 6.3 Eventos SSE (borrador)

```
session.message.created
session.message.updated
session.part.appended
session.part.updated
session.tool_call.updated
session.message.completed
session.message.failed
server.connected
```

Nombres finales se cierran en el change de protocol. No inventar eventos extra “por si acaso”.

---

## 7. Adapter

### 7.1 Trait (intención)

El adapter traduce el modelo de Circulo al proveedor y viceversa.

Responsabilidades:

- Descubrir si el proveedor está disponible.
- Abrir / reanudar una conversación remota si el proveedor tiene sesión propia.
- Enviar el turno del usuario.
- Emitir un stream de eventos normalizados (`TextDelta`, `ToolCallStarted`, `ToolCallUpdated`, `Completed`, `Failed`).
- Mapear errores a `AdapterError`.

No responsabilidades (MVP):

- Persistencia de Circulo.
- Render.
- Autenticación de usuario Circulo.
- Plugin loading dinámico.

### 7.2 OpenCode (MVP)

Hechos conocidos por investigación previa, **a verificar de nuevo antes de implementar**:

- OpenCode expone un server HTTP headless: `opencode serve`.
- Documentación: `https://opencode.ai/docs/server/`
- Hay endpoints de project, session y un stream `GET /event` (SSE).

El adapter **debe** basarse en esa API real (o la que esté vigente al momento del change), no en conjeturas.

Decisiones abiertas del adapter:

1. ¿Circulo lanza `opencode serve` o se conecta a uno ya levantado?
2. ¿Cómo se mapea `Project`/`Session` de Circulo a project/session de OpenCode?
3. ¿El working directory lo elige el usuario o es implícito?
4. ¿Qué subset de eventos OpenCode se traduce a `MessagePart`?
5. Auth del server OpenCode (`OPENCODE_SERVER_PASSWORD`) — ¿el daemon la gestiona?

Sin esas respuestas investigadas, no se escribe el adapter.

---

## 8. Frontend (GPUI)

### 8.1 Responsabilidades

- Window chrome custom (hidden title bar).
- Layout `AppShell` = Sidebar + main (header, messages, composer).
- Cliente HTTP/SSE contra el daemon.
- Reducer de estado de UI a partir de eventos.
- Render de `MessagePart` vía componentes propios.

### 8.2 Restricciones

- Sin WebView como superficie principal del chat.
- Sin animaciones no nativas.
- Componentes inspirados en Shadcn/Base UI, implementados en GPUI.
- Iconos Heroicons. Pipeline de assets se define en el primer change de UI.
- El render de Markdown debe ser incremental y no bloquear el input.

### 8.3 Window / macOS

- Title bar oculto / transparente.
- Traffic lights posicionados en el Sidebar, alineados con el botón hide/show.
- Sidebar colapsado = rail mínimo que conserva traffic lights + botón de expandir. Nunca migran al header.

### 8.4 Arranque de procesos

Dos procesos Circulo, cerrados:

1. El usuario abre `circulo-app`.
2. La app arranca o reutiliza `circulo-daemon` en localhost (supervisión: spawn + healthcheck + restart suave). El usuario no lanza el daemon a mano.
3. El daemon, cuando hay que hablar con el agente, usa el adapter contra OpenCode (`opencode serve`).

**Quién lanza OpenCode** sigue abierto (el daemon lo spawnea vs se conecta a uno existente). No es un proceso de Circulo.

### 8.5 i18n

- Default locale: `en`.
- Todas las cadenas de UI (incluyendo `No project`, empty states, errores humanos) viven en archivos de locale.
- Lookup por clave; fallback a `en` si falta una traducción.
- El crate de UI depende de `circulo-i18n`, no de literales.
- Formato del catálogo se elige en el change de i18n (Fluent o JSON). No hardcodear mientras tanto.

---

## 9. Requisitos no funcionales técnicos

| ID | Requisito | Notas |
| --- | --- | --- |
| TRD-NFR-01 | Acciones locales < 100 ms percibidos | Abrir sidebar item, focus composer |
| TRD-NFR-02 | El stream no bloquea el thread de UI | Parseo y apply de eventos fuera del render crítico |
| TRD-NFR-03 | Memoria acotada en chats largos | Ventana de render / virtualización si hace falta |
| TRD-NFR-04 | Daemon solo localhost | No bind 0.0.0.0 |
| TRD-NFR-05 | Sin secretos en el repo | API keys de proveedores no se hardcodean |
| TRD-NFR-06 | Builds reproducibles en macOS | Documentar toolchain (rustc, targets) en el change de scaffold |
| TRD-NFR-07 | Tests unitarios de core/protocol/adapter | Obligatorios por change |
| TRD-NFR-08 | Logs estructurados en daemon | Niveles; la UI no muestra logs crudos al usuario |
| TRD-NFR-09 | i18n | Cero strings de UI fuera de locale files |

---

## 10. Seguridad (MVP)

Alcance pequeño y local:

- El daemon no es un servicio de red pública.
- HTTPS local: investigar amenaza real vs complejidad. Si HTTPS añade fricción de certificados peor que el beneficio en localhost, **preguntar** antes de insistir. El Project Definition dice HTTPS; no se ignora, se valida.
- No enviar datos de Circulo a ningún backend propio (no existe).
- OpenCode puede hablar con LLMs de terceros: eso es del proveedor, no de Circulo. La UI no debe fingir que “todo es local” si el agente sale a la red.
- Settings no pide pegar API keys en el MVP salvo que OpenCode lo exija y se investigue.

---

## 11. Observabilidad

MVP:

- Logs del daemon en archivo local de Application Support.
- `GET /v1/health` para la UI.
- No telemetry remota.

---

## 12. Tooling

| Uso | Herramienta |
| --- | --- |
| App y daemon | Rust, Cargo workspace |
| Scripts | Bun + TypeScript |
| Specs | OpenSpec (`openspec/`) |
| Control de versiones | Git, branch por feature/change |

No introducir Electron, Tauri, React o un frontend web “temporal”.

---

## 13. Testing técnico

| Capa | Qué se prueba | Cómo |
| --- | --- | --- |
| `core` | Invariantes de entidades, transiciones de status | Unit |
| `protocol` | Serde roundtrip, versionado, eventos | Unit |
| `persist` | CRUD, integridad referencial, búsqueda | Unit + fs temp |
| `adapter-opencode` | Mapping de eventos, errores de unreachability | Unit con fixtures; contract tests si hay sandbox |
| `daemon` | HTTP + SSE con adapter fake | Integration |
| `app` | Flujos visuales | Prueba manual obligatoria antes de commit (ver AGENTS.md) |

Un adapter fake (`circulo-adapter-fake`) es deseable para desarrollar UI sin OpenCode. Requiere permiso y un change propio o una tarea explícita en el change de chat.

---

## 14. Riesgos técnicos

| Riesgo | Impacto | Mitigación |
| --- | --- | --- |
| GPUI tiene ecosistema chico | Componentes lentos de construir | Scope de UI mínimo; no reinventar un design system entero en el primer change |
| API de OpenCode cambia | Adapter se rompe | Aislar 100% en su crate; fixtures de contrato |
| HTTPS local es doloroso en macOS | Fricción de setup | Investigar; no improvisar |
| Streaming + Markdown caro | UI trabada | Incremental parse; tests de carga cualitativos |
| Lifecycle de dos procesos | App debe spawn/reusar daemon | Healthcheck + restart; diseñar en `scaffold-workspace` |
| OpenCode caído | Chat no funciona | Banner humano; no fingir stream |

---

## 15. Decisiones abiertas (bloqueantes para código)

1. Estrategia HTTPS/certs en localhost (app ↔ daemon).
2. Cómo se descubre y lanza OpenCode (`opencode serve`).
3. Mapeo de identidad Session Circulo ↔ session OpenCode.
4. Pipeline de Heroicons → GPUI.
5. Virtualización del message list: ¿desde el día 1 o cuando duela?
6. Versión mínima de macOS.
7. Bundle ID y nombre de app (`app.circulo` u otro).
8. Formato de catálogo i18n (Fluent vs JSON).
9. Supervisión concreta app → daemon (socket, pidfile, puerto fijo).

Cerradas el 16 ago 2026: SQLite; dos procesos Circulo; `project_id` nullable; app no habla con OpenCode; UI `en` + locales.

Hasta investigar y pedir permiso, estas decisiones no se “resuelven” en un commit.

---

## 16. Relación con OpenSpec

- Este TRD es contexto de proyecto, no un change.
- El primer código debe nacer de un change (p. ej. `scaffold-workspace` o `mvp-app-shell`) con proposal, specs, design y tasks.
- Specs de capacidad viven en `openspec/specs/` cuando se archivan changes.
- Si el TRD y un spec divergen, se actualiza vía change, no se “arregla callado”.
