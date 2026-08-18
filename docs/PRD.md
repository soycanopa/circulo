# Circulo — Product Requirements Document (PRD)

| Campo | Valor |
| --- | --- |
| Producto | Circulo |
| Versión | 0.4 (derivado de Project Definition 0.8) |
| Fecha | 16 de agosto de 2026 |
| Estado | Pre-MVP / planificación |
| Fuente | `Circulo-Project-Definition.md` |
| Audiencia | Producto, diseño, ingeniería |

Este documento define **qué** se construye y **por qué**. No define implementación. El TRD, UX/UI, Flows e Implementation son complementarios. Cualquier feature nueva entra por OpenSpec, no por editar este PRD de forma aislada.

---

## 1. Resumen

Circulo es un cliente de escritorio nativo, rápido y visualmente cuidado, para conversar con agentes de IA. El usuario objetivo no es un developer experto: es alguien de marketing, producto, diseño u otro perfil no técnico que quiere **crear** (textos, ideas, landings, requisitos, assets) sin pelearse con herramientas hechas para programadores.

Filosofía:

> Las herramientas suficientes para hacer el trabajo genial.

Profundidad y calidad por encima de cantidad de features.

---

## 2. Problema

| Problema | Consecuencia |
| --- | --- |
| Los clientes de agentes están pensados para developers | Personas no técnicas se sienten perdidas o excluidas |
| El streaming suele ser texto plano | Difícil de seguir, poco usable para trabajo creativo |
| Poca organización visual por proyecto | El trabajo se pierde en una lista de chats |
| Clientes pesados (p. ej. Electron) | Se sienten lentos y caros en recursos |

Circulo existe para ofrecer una experiencia **rápida, limpia y estructurada**, orientada a crear, no a configurar.

---

## 3. Visión de producto

El lugar donde una persona de marketing, producto o diseño conversa con un agente, organiza el trabajo por proyectos y obtiene resultados estructurados y útiles, sin conocimiento técnico profundo.

No es un IDE. No es un terminal disfrazado. No es un panel de 100 integraciones.

---

## 4. Personas

| Persona | Qué hace | Qué necesita de Circulo |
| --- | --- | --- |
| Marketing | Campañas, copy, landings, contenidos | Generar y refinar textos e ideas rápido, con resultados claros |
| Producto | Features, requisitos, flujos | Explorar ideas, estructurar requisitos, prototipar narrativa |
| Diseño | Copy, estructura, a veces código ligero | Apoyo creativo y generación de contenido sin fricción |
| No técnico en general | Quiere usar agentes sin ser developer | Interfaz simple, estados comprensibles, cero jerga innecesaria |

**Prioridad de diseño:** claridad y accesibilidad para no técnicos. Un developer puede usarlo, pero no es el centro de las decisiones de UX.

---

## 5. Principios de producto

1. Claridad sobre complejidad.
2. Herramientas suficientes, no excesivas. Cada feature debe justificar su valor para el acto de crear.
3. Velocidad y responsividad como prioridad absoluta.
4. Streaming de alta calidad: estructurado y legible *mientras* se genera.
5. Organización por proyecto como unidad natural de trabajo.
6. Modularidad: crecer sin volverse un monstruo.
7. Experiencia visual cuidada: animaciones nativas, tipografía limpia, componentes bien resueltos.

Si una propuesta viola un principio, se rechaza o se recorta, aunque “se vea bien” o “sea fácil de añadir”.

---

## 6. Metas y no-metas del MVP

### 6.1 Objetivo del MVP

Una app usable, rápida y bonita en **macOS** que permita conversar de forma rica con **OpenCode**, organizar sesiones por proyecto y ver respuestas estructuradas (markdown, tool calls, diffs, tasks básicos).

### 6.2 En alcance

- Un solo proveedor: OpenCode.
- Chat rico: Markdown + tool calls + diffs + task lists básicos.
- Sidebar con secciones **Today** y **Earlier** (actividad en el día local vs días anteriores).
- Una sesión nueva nace **sin proyecto** en la carpeta especial de Circulo (`Sessions`), salvo que el usuario elija una carpeta de proyecto en el composer.
- El composer permite elegir una carpeta de proyecto **antes** de avanzar con el chat. Si no se elige, la sesión queda en la carpeta especial `Sessions` / **Without Folder**.
- Borrar un proyecto borra también sus sesiones (están vinculadas a esa carpeta).
- Settings incluye una sección de **proyectos archivados**.
- Búsqueda y filtrado básico de sesiones.
- Daemon local simple (conversar + stream).
- Frontend nativo GPUI.
- Ventana sin title bar nativo. Traffic lights y el botón de ocultar/mostrar sidebar viven **alineados en el Sidebar** (también con el sidebar colapsado: rail mínimo).
- UI en **inglés**, con todas las cadenas en un catálogo de locale (i18n listo para más idiomas).
- Dark theme por defecto.
- Indicador de agente por sesión (aunque solo exista OpenCode).

### 6.3 Fuera de alcance (explícito)

- Windows y Linux.
- Otros proveedores (Cursor, Claude Code, Grok, Codex, etc.).
- Preguntas interactivas avanzadas (existen en el modelo de datos; no se implementan en el MVP).
- Sistema de plugins completo.
- Colaboración, compartición, multi-usuario, cloud sync.
- Temas avanzados y personalización profunda.
- Cambio de worktree / working directory por sesión (fuera de alcance ahora).
- Flujos multi-agente.
- Marketplace de adapters.
- Autenticación de cuenta Circulo.
- Facturación.

### 6.4 Criterios de éxito

- Una persona de marketing o producto abre Circulo y conversa con OpenCode sin fricción.
- El streaming se siente estructurado y agradable, no como un muro de texto.
- Organizar sesiones por proyecto es intuitivo.
- La app se siente rápida y responsiva.
- Se puede añadir el siguiente proveedor más adelante sin reescribir el frontend ni el contrato del daemon.

---

## 7. Requisitos funcionales

Los IDs son estables. Un cambio de OpenSpec debe referenciarlos.

### 7.1 Arranque y ventana

| ID | Requisito | Prioridad |
| --- | --- | --- |
| PRD-APP-01 | Circulo es una app de escritorio nativa para macOS. | P0 |
| PRD-APP-02 | La ventana no usa title bar nativo. Traffic lights (cerrar, minimizar, maximizar) viven dentro del Sidebar. | P0 |
| PRD-APP-03 | El usuario puede colapsar y expandir el Sidebar. | P0 |
| PRD-APP-04 | Al colapsar el Sidebar queda un rail mínimo. Traffic lights y el botón de mostrar/ocultar permanecen en el Sidebar, alineados. Nunca migran al header. | P0 |
| PRD-APP-06 | Toda cadena visible al usuario vive en un catálogo de locale. Locale por defecto: `en`. | P0 |
| PRD-APP-05 | La app debe arrancar y quedar usable sin configuración técnica visible. | P0 |

### 7.2 Proyectos

| ID | Requisito | Prioridad |
| --- | --- | --- |
| PRD-PRJ-01 | El usuario puede crear un proyecto con nombre. | P0 |
| PRD-PRJ-02 | Un proyecto puede tener descripción y color opcionales. | P1 |
| PRD-PRJ-03 | El usuario puede renombrar un proyecto. | P0 |
| PRD-PRJ-04 | El usuario puede **archivar** un proyecto. El proyecto y sus sesiones salen de las vistas principales y se consultan en Settings → Archived projects. | P0 |
| PRD-PRJ-05 | Un proyecto es opcional. Las sesiones pueden existir sin proyecto. | P0 |
| PRD-PRJ-06 | Circulo tiene una carpeta especial de sistema, `Sessions`, para sesiones sin proyecto. No es un proyecto creado por el usuario. | P0 |
| PRD-PRJ-07 | **Borrar** un proyecto borra en cascada todas sus sesiones. Están vinculadas a esa carpeta. Requiere confirmación explícita. | P0 |
| PRD-PRJ-08 | Archivar y borrar no son lo mismo. Archivar conserva; borrar destruye. | P0 |

### 7.3 Sesiones

| ID | Requisito | Prioridad |
| --- | --- | --- |
| PRD-SES-01 | “New session” crea una sesión **sin proyecto** y la abre. Queda en la carpeta especial `Sessions` hasta que el composer asigne una carpeta. | P0 |
| PRD-SES-02 | El usuario puede abrir una sesión existente y ver su historial. | P0 |
| PRD-SES-03 | El usuario puede asignar una sesión a un proyecto, moverla a otro, o dejarla sin proyecto (composer u otra acción). | P0 |
| PRD-SES-04 | El usuario puede archivar una sesión. | P1 |
| PRD-SES-05 | El título de la sesión es visible y editable. Puede generarse automáticamente. | P1 |
| PRD-SES-06 | Cada sesión muestra qué agente usa. En el MVP: OpenCode. | P0 |
| PRD-SES-07 | El Sidebar lista sesiones en dos secciones: **Today** (actividad en el día local actual) y **Earlier** (actividad en días anteriores). | P0 |
| PRD-SES-08 | El usuario puede buscar y filtrar sesiones por texto en ambas secciones. | P0 |
| PRD-SES-09 | Cada item muestra: nombre, duración relativa de la sesión (derecha), y carpeta (nombre del proyecto o **Without Folder**). | P0 |

### 7.4 Chat

| ID | Requisito | Prioridad |
| --- | --- | --- |
| PRD-CHT-01 | El usuario puede escribir un mensaje y enviarlo al agente de la sesión. | P0 |
| PRD-CHT-02 | La respuesta se transmite en streaming y se renderiza de forma incremental. | P0 |
| PRD-CHT-03 | El contenido de texto se renderiza como Markdown de alta calidad (encabezados, listas, tablas, código, énfasis). | P0 |
| PRD-CHT-04 | El inline code y los paths se distinguen visualmente (estilo pill/badge). | P1 |
| PRD-CHT-05 | Las tool calls se muestran como cards con nombre, estado y resultado. | P0 |
| PRD-CHT-06 | Un resultado de tipo diff se puede revisar de forma cómoda. | P0 |
| PRD-CHT-07 | El agente puede mostrar una lista de tareas con estado (pendiente, en progreso, completada, cancelada). | P0 |
| PRD-CHT-08 | El usuario ve estados claros: enviando, generando, completado, error. | P0 |
| PRD-CHT-09 | El usuario puede cancelar una generación en curso. | P1 |
| PRD-CHT-10 | Preguntas interactivas estructuradas quedan fuera del MVP (el modelo las prevé). | — |
| PRD-CHT-11 | Mientras el agente genera, el composer lo indica y evita envíos ambiguos. | P0 |
| PRD-CHT-12 | El composer incluye un selector de carpeta de proyecto, usable **solo al iniciar el chat** (antes del primer envío). Elegir una asigna la sesión a ese proyecto. | P0 |
| PRD-CHT-13 | Si el selector queda en vacío / **Without Folder**, la sesión permanece en la carpeta especial `Sessions`. | P0 |
| PRD-CHT-14 | Después del primer envío el selector queda bloqueado. No se cambia de proyecto a mitad de sesión. | P0 |

### 7.5 Agente y daemon

| ID | Requisito | Prioridad |
| --- | --- | --- |
| PRD-AGT-01 | El único proveedor del MVP es OpenCode. | P0 |
| PRD-AGT-02 | Si OpenCode no está disponible, el usuario ve un error comprensible, no un stack trace. | P0 |
| PRD-AGT-03 | El frontend no habla con OpenCode directamente; habla con el daemon local. | P0 |
| PRD-AGT-04 | El daemon gestiona conexión, streaming y API local. | P0 |
| PRD-AGT-05 | Un usuario no técnico no debe tener que “levantar un servidor” a mano en el camino feliz. | P0 |

### 7.6 Settings

| ID | Requisito | Prioridad |
| --- | --- | --- |
| PRD-SET-01 | Existe un punto de entrada a Settings en el Sidebar. | P0 |
| PRD-SET-02 | Settings del MVP incluye lo imprescindible para conversar (p. ej. estado de OpenCode) y no es un panel de power-user. | P1 |
| PRD-SET-03 | Settings tiene una sección **Archived projects** que lista los proyectos archivados. | P0 |
| PRD-SET-04 | Desde Archived projects el usuario puede **restaurar** un proyecto. El proyecto vuelve a Active; sus sesiones reaparecen en el sidebar (Today/Earlier) con su etiqueta de carpeta. | P0 |

---

## 8. Requisitos no funcionales (producto)

Detalle técnico en el TRD. A nivel de producto:

| ID | Requisito |
| --- | --- |
| PRD-NFR-01 | La UI debe sentirse instantánea en acciones locales (abrir sesión, scroll, tipear). |
| PRD-NFR-02 | El primer token visible del stream no debe hacer que la UI se congele. |
| PRD-NFR-03 | La app no debe sentirse como un browser empaquetado. |
| PRD-NFR-04 | Errores en lenguaje humano. Cero jerga de CLI en el camino principal. |
| PRD-NFR-05 | Los datos de proyectos, sesiones y mensajes viven en local (SQLite). El MVP no sincroniza a la nube. |
| PRD-NFR-06 | La arquitectura debe permitir un segundo adapter sin reescribir el chat. |
| PRD-NFR-07 | La UI está en inglés. Ningún string de UI se hardcodea: todo pasa por locale. |

---

## 9. Experiencia esperada (resumen)

- Arrancar → ver la lista `Sessions` (o empty state) → New session → escribir → ver una respuesta estructurada.
- Tool calls como tarjetas elegantes, no logs de terminal.
- Cambiar de proyecto/sesión es un gesto natural, no un ritual.
- Animaciones nativas, sutiles, nunca decorativas a costa de latencia.

Detalle en `docs/UX-UI.md` y `docs/FLOWS.md`.

---

## 10. Roadmap de producto

| Versión | Foco |
| --- | --- |
| MVP | OpenCode + chat rico + sesiones por proyecto + daemon simple + macOS |
| v0.2 | Usabilidad a partir de feedback de no técnicos |
| v0.3 | Un proveedor más, según demanda |
| v0.4 | Preguntas interactivas + mejor Tasks |
| v0.5 | Sistema de adapters más maduro |
| Futuro | Más agentes, posibles flujos multi-agente |

El roadmap no autoriza implementación. Cada incremento es un change de OpenSpec.

---

## 11. Decisiones de producto

| Tema | Estado | Decisión |
| --- | --- | --- |
| Público | Cerrada | Marketing, producto, diseño, no técnicos |
| Filosofía | Cerrada | Herramientas suficientes que funcionen muy bien |
| Plataforma MVP | Cerrada | macOS primero |
| Proveedor MVP | Cerrada | Solo OpenCode |
| UI framework | Cerrada | GPUI nativo |
| Title bar | Cerrada | Sin title bar nativo; traffic lights + hide alineados en el Sidebar (rail al colapsar) |
| Sesión nueva | Cerrada | Sin proyecto; carpeta especial de sistema `Sessions` |
| Vistas Sidebar | Cerrada | **Today** / **Earlier** por actividad local |
| Composer | Cerrada | Selector de carpeta solo al iniciar el chat; después locked |
| Worktree | Cerrada (fuera) | No se implementa cambio de worktree ahora |
| Borrar proyecto | Cerrada | Cascada: se van todas sus sesiones. Confirmación obligatoria |
| Archivar proyecto | Cerrada | Conserva; Settings → Archived projects; **se puede restaurar** |
| Idioma UI | Cerrada | Inglés (`en`) + infraestructura de locales |
| Temas | Cerrada para MVP | Dark por defecto; sin personalización profunda |
| Preguntas interactivas | Cerrada para MVP | Fuera de alcance de implementación |
| Multi-agente | Abierta | Un agente por sesión en el modelo; un solo provider real |

---

## 12. Decisiones abiertas (no asumir)

Preguntas que aún condicionan trabajo futuro:

3. **Settings más allá de OpenCode + archived/restore:** ¿también modelo / extras? (worktree no). *Parcialmente resuelto: panel Models ya existe; falta decidir alcance final.*

5. **Orden de la lista plana:** ¿por `last_message_at` descendente? *Implementado así en sidebar; pendiente confirmación formal en spec.*

Cerradas el 16 ago 2026: restore desde Settings; selector locked tras el primer send; sin worktree; sidebar Today/Earlier.

Cerradas el 18 ago 2026:

1. **Título automático:** OpenCode reemplaza solo títulos por defecto (`New session`); renombres manuales no se pisan.
2. **Cancelar stream:** implementado (botón Stop en composer).
4. **Branding en sidebar:** solo icono/rail en MVP; sin wordmark.
6. **Confirmación al restaurar:** restore inmediato en Settings → Archived; delete sigue pidiendo confirmación.

---

## 13. Métricas cualitativas del MVP

No hay analytics en el MVP. El éxito se valida de forma manual:

- Una persona no técnica completa el flujo “abrir → preguntar → entender la respuesta” sin ayuda.
- Puede encontrar una sesión de ayer y continuar.
- Puede explicar qué está pasando cuando hay un tool call (sin que le digan “es una tool call”).
- No percibe lag al tipear ni al scrollear un chat medio.

---

## 14. Relación con otros documentos

| Documento | Rol |
| --- | --- |
| `Circulo-Project-Definition.md` | Fuente de idea (v0.8) |
| `docs/TRD.md` | Cómo se construye |
| `docs/UX-UI.md` | Superficie visual y componentes |
| `docs/FLOWS.md` | Flujos y estados |
| `docs/IMPLEMENTATION.md` | Fases, módulos, Definition of Done |
| `AGENTS.md` | Contrato de trabajo para agentes e ingeniería |
| `openspec/` | Specs ejecutables por change |
