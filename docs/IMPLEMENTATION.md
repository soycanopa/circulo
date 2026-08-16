# Circulo — Implementation Plan

| Campo | Valor |
| --- | --- |
| Producto | Circulo |
| Versión | 0.4 |
| Fecha | 16 de agosto de 2026 |
| Estado | Plan. Cero código de producto hasta un change OpenSpec aprobado y permiso explícito. |

Este documento dice **en qué orden** se construye y **qué significa hecho**. No se implementa desde aquí en un solo PR.

---

## 1. Regla de arranque

Circulo se construye **solo con specs**.

```
Idea / duda  →  openspec-explore (si hace falta)
             →  openspec-propose  (proposal, specs, design, tasks)
             →  revisión humana
             →  permiso explícito
             →  openspec-apply-change en un branch de feature
             →  tests automáticos + prueba manual
             →  commits granulares
             →  merge
             →  openspec-archive-change cuando el change esté completo
```

Prohibido:

- Empezar un crate “para ir adelantando” sin change.
- Implementar el MVP entero en un solo change monstruo.
- Commitear para “dejar constancia” sin prueba manual del slice.
- Resolver decisiones abiertas del PRD/TRD/UX en silencio.

---

## 2. Estrategia modular

Cada incremento deja el sistema compilable y, en cuanto exista app, usable en un subset.

Capas (de adentro hacia afuera):

1. `core` + `protocol` — tipos y contrato.
2. `persist` — datos locales.
3. `adapter` trait + `adapter-fake` — stream determinista para UI y tests.
4. `daemon` — HTTP/SSE sobre fake.
5. `app` — shell GPUI hablando con daemon.
6. Chat render (markdown, tool cards, tasks).
7. `adapter-opencode` — reemplaza fake en el camino real.
8. Pulido de organización (search, move, empty/error reales con OpenCode).

El adapter real **no** es el primer ladrillo. Si se empieza por OpenCode, la UI queda rehén de un sistema externo.

---

## 3. Changes OpenSpec previstos (MVP)

Nombres tentativos. Cada uno = un branch `feature/<nombre>`. No crearlos todos ahora.

| Orden | Change | Entrega | Depende de |
| --- | --- | --- | --- |
| 1 | `scaffold-workspace` | Cargo workspace, dos binarios (`circulo-app`, `circulo-daemon`), toolchain | macOS mínimo sigue abierto |
| 2 | `core-and-protocol` | Entidades (`project_id` opcional), serde, eventos, `api_version` | 1 |
| 3 | `local-persistence` | SQLite + migraciones; `project_id` nullable; **ON DELETE CASCADE**; `Project.status` Active/Archived | 2 |
| 4 | `adapter-trait-and-fake` | Trait + fake que emite texto, tool call y tasks | 2 |
| 5 | `local-daemon-api` | HTTPS/HTTP localhost + SSE + health; spawn/reuso del daemon | 3, 4 + decisión certs |
| 6 | `app-shell-window` | GPUI, hidden title bar, TLs + hide alineados, rail colapsado, i18n `en` | 1 |
| 7 | `sessions-sidebar` | ViewSwitcher, persistencia de vista (fallback Sessions), Groups vacío = New project | 5, 6 |
| 8 | `composer-and-message-stream` | Enviar, selector de carpeta solo pre-primer-send (luego locked) | 5, 7 |
| 9 | `rich-message-render` | Markdown, ToolCallCard, Diff, TaskList | 8 |
| 10 | `adapter-opencode` | Detección, mapping, errores humanos | 5 + investigación de API vigente |
| 11 | `mvp-hardening` | Empty/error, health, Settings + archive/restore, confirmación de delete, E2E | 9, 10 |

Un change puede partirse si las tasks superan un tamaño razonable. Unir dos de la lista requiere permiso.

No hay change “implementar el MVP”.

---

## 4. Definition of Done por change

Un change no está hecho cuando “compila en mi máquina”. Está hecho cuando:

1. Existe proposal + specs + design + tasks en OpenSpec, alineados entre sí.
2. El código cumple las specs (no el recuerdo del chat).
3. Tests automáticos de la capa tocada pasan.
4. Se hizo **prueba manual** del flujo afectado (si hay UI o API observable).
5. No se introdujeron decisiones abiertas “resueltas por conveniencia”.
6. El diff es del tamaño del change, no un refactor oportunista.
7. Recién entonces: commits granulares en el branch de la feature.

Archive del change: solo cuando las tasks están hechas y el usuario lo pide o confirma.

---

## 5. Orden de trabajo recomendado en un change

1. Leer PRD / TRD / UX / Flows y el change actual.
2. Investigar (código, API, GPUI, OpenCode). Anotar hallazgos.
3. Si aparece ambigüedad de producto o arquitectura: **parar y preguntar**.
4. Pedir permiso para el plan de implementación concreto (archivos, crates, APIs).
5. Implementar el mínimo que cierra las tasks.
6. Probar.
7. Commits granulares.
8. Resumir qué quedó abierto.

---

## 6. Convención de branches y commits

### Branch

- `main` — estable. No se trabaja encima salvo hotfix trivial acordado.
- `feature/<openspec-change-name>` — una feature / un change.
- No mezclar dos changes en el mismo branch.

### Commits

- Granulares: un propósito por commit (tipos, persist, un componente, un test).
- Conventional Commits: `feat:`, `fix:`, `refactor:`, `test:`, `docs:`, `chore:`.
- Prohibido: `WIP` eterno, dumps de 30 archivos sin relación, commits de archivos generados irrelevantes.
- **No hay commit sin prueba manual** del slice que el commit introduce, cuando ese slice es observable. Un commit solo de tipos puede validarse con tests unitarios; un commit de UI no.

Ejemplos buenos:

- `feat(core): add Session and Message entities`
- `test(protocol): roundtrip tool call events`
- `feat(app): render tool call card success state`

Ejemplos malos:

- `feat: mvp`
- `updates`
- `fix stuff`

---

## 7. Testing

| Tipo | Cuándo | Obligatorio |
| --- | --- | --- |
| Unit (`core`, `protocol`, `persist`, mapping del adapter) | Desde el primer change de esa capa | Sí |
| Integration (daemon + fake adapter) | Desde `local-daemon-api` | Sí |
| Contract / fixtures OpenCode | Desde `adapter-opencode` | Sí, con fixtures grabadas; no depender de red en unit |
| Manual UI | Cualquier change que toque GPUI o un flujo de `docs/FLOWS.md` | Sí, antes del commit |
| Performance informal | Stream largo, markdown pesado | En `rich-message-render` y hardening |

No se mide el éxito por cantidad de tests de adorno. Se mide por invariantes reales (serde, status transitions, “no mezclar sessions”, adapter unreachable).

---

## 8. Scaffold mínimo (cuando se apruebe el change 1)

Intención, no estructura creada todavía:

```
circulo/
  Cargo.toml                 # workspace
  crates/
    circulo-core/
    circulo-protocol/
    circulo-adapter/
    circulo-adapter-fake/
    circulo-adapter-opencode/
    circulo-persist/
    circulo-daemon/
    circulo-app/
    circulo-i18n/
    circulo-markdown/
  docs/
  openspec/
  AGENTS.md
  Circulo-Project-Definition.md
```

Scripts Bun solo si hay una tarea real (codegen de iconos, etc.), no un `package.json` vacío.

---

## 9. Qué no se construye “de paso”

- Plugin loader genérico.
- Temas.
- QuestionCard completo.
- Telemetría.
- CI multiplataforma.
- Windows/Linux.
- Abstracciones para 5 proveedores futuros. El trait debe admitir un segundo adapter; no se diseña un marketplace.

---

## 10. Primer paso concreto (siguiente conversación)

No está autorizado todavía. El siguiente movimiento correcto es:

1. El change `scaffold-workspace` ya puede proponerse: dos procesos y SQLite están cerrados. Siguen abiertos macOS mínimo, certs y spawn de OpenCode.
2. Pedir explícitamente: “propón el change `scaffold-workspace`”.
3. Recién entonces, `openspec-propose`.

Hasta ese pedido, el repositorio permanece en documentación + git inicializado.

---

## 11. Trazabilidad

| Pregunta | Dónde se responde |
| --- | --- |
| ¿Debemos construir esto? | PRD + OpenSpec proposal |
| ¿Qué debe pasar? | OpenSpec specs + Flows |
| ¿Cómo se ve? | UX/UI |
| ¿Cómo se implementa? | TRD + OpenSpec design |
| ¿En qué tareas? | OpenSpec tasks |
| ¿Cómo se trabaja? | `AGENTS.md` |
