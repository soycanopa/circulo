# Circulo — Roadmap de producto

Documento de referencia para decidir qué traer de repos relacionados al **Circulo actual** (Tauri v2 + Rust + ACP stdio + React 19).

**Fuentes analizadas:**

| Repo | Stack | Relación con nosotros |
|------|-------|------------------------|
| [soycanopa/circulo](https://github.com/soycanopa/circulo) | Electron + OpenCode SDK (HTTP/SSE) + monorepo Turborepo | Fork previo tuyo de Palot; misma marca, distinta arquitectura |
| [openchamber/openchamber](https://github.com/openchamber/openchamber) | Desktop + Web/PWA + VS Code ext; OpenCode server | Referente de UX madura; 2k+ commits, comunidad activa |

**Circulo hoy (v0.1):** cliente ACP nativo con chat streaming, permisos, tool calls, diffs inline, plan mode, multi-sesión, sidebar de proyectos, frosted glass macOS. **No** tiene settings, review panel, git integrado, automations, ni capa HTTP al agente.

**Criterio de priorización:** de **más compleja → más sencilla**, ajustado a nuestro stack ACP/Tauri (no copiar Electron/SDK tal cual si requiere re-arquitectura).

---

## Leyenda

| Etiqueta | Significado |
|---------|-------------|
| 🟢 **ACP-ready** | Encaja con protocolo actual sin cambiar backend |
| 🟡 **Híbrido** | Parte UI portable; parte requiere APIs extra del agente o capa Rust nueva |
| 🔴 **Re-arquitectura** | Conflicto con modelo ACP-only o esfuerzo multi-mes |
| 📦 **Port directo** | Código/paquete reutilizable del repo Electron Circulo |

---

## Nivel 1 — Arquitectura y plataforma (máxima complejidad)

### 1.1 Motor de automatizaciones programadas
**Fuente:** soycanopa/circulo  
**Complejidad:** 🔴 muy alta  
**Estado nuestro:** no existe

- Runs recurrentes con RRule, cola `pending_review`, retry con backoff, auto-archive sin cambios.
- En Electron vive en el **main process** + scheduler + spawn de sesiones OpenCode.
- En Tauri: nuevo módulo Rust (`scheduler.rs`), persistencia SQLite, UI de reglas, integración ACP para turnos en background.

**Recomendación:** fase 3+. No bloquea MVP; alto valor diferenciador.

---

### 1.2 Worktrees aislados + sesiones paralelas por rama
**Fuente:** ambos  
**Complejidad:** 🔴 muy alta  
**Estado nuestro:** no existe

- OpenChamber: multi-agent desde un prompt con worktrees; merge con resolución de conflictos.
- Circulo Electron: página de settings de worktrees para automations/experimentos.
- Requiere: gestión git en Rust (`git2` o comandos), UI de worktrees, política de cwd por sesión ACP.

**Recomendación:** después del review panel y git básico.

---

### 1.3 Cliente dual ACP + OpenCode SDK (HTTP)
**Fuente:** implícito al comparar los tres proyectos  
**Complejidad:** 🔴 muy alta  
**Estado nuestro:** solo ACP

- soycanopa/circulo y OpenChamber asumen **servidor OpenCode** con SDK rico (undo, compact, attachments, MCP admin, etc.).
- Muchas features “fáciles” en esos repos **no existen en ACP** o están incompletas.

**Recomendación:** **no** hacer dual stack a corto plazo. Priorizar gaps vía ACP + contribuir al protocolo. Reevaluar solo si ACP no cubre undo/attachments en 6 meses.

---

### 1.4 Acceso remoto: túneles, SSH, PWA, instancias múltiples
**Fuente:** OpenChamber  
**Complejidad:** 🔴 alta  
**Estado nuestro:** app desktop local únicamente

- Cloudflare tunnel, QR onboarding, `openchamber://` deep links, SSH port forwarding, host switcher.
- Contradice parcialmente la apuesta Tauri ligera + ACP local.

**Recomendación:** fuera de scope v1. Posible spin-off “Circulo Remote” mucho más adelante.

---

### 1.5 Wizard de migración Claude Code / Cursor
**Fuente:** soycanopa/circulo (`packages/configconv`)  
**Complejidad:** 🟡 alta · 📦 port parcial  
**Estado nuestro:** no existe

- Convierte MCP, agents, commands, rules (`CLAUDE.md` → `AGENTS.md`), hooks e historial (`state.vscdb`).
- El paquete `configconv` es **reutilizable** independiente del shell Electron.

**Recomendación:** portar `configconv` como crate CLI o invocación Tauri; wizard en React. Alto impacto onboarding.

---

## Nivel 2 — Flujos de producto grandes

### 2.1 Review panel dedicado (diffs de sesión)
**Fuente:** soycanopa/circulo (principal), OpenChamber  
**Complejidad:** 🟡 alta  
**Estado nuestro:** diffs inline en chat; sin panel lateral

- Virtualización (TanStack Virtual), worker pool para Shiki, smart collapse de lockfiles/archivos enormes.
- Comentarios por línea → inyectar feedback al chat (nosotros ya tenemos patrón similar en **plan comment**).

**Recomendación:** **prioridad alta** — alinea con visión Palot; reutilizar `@pierre/diffs` + virtualización.

---

### 2.2 Git integrado: commit, push, PR
**Fuente:** ambos  
**Complejidad:** 🟡 alta  
**Estado nuestro:** no existe

- Diálogo commit/branch/push/PR GitHub desde la app.
- Rust: `git` subprocess + API GitHub (token en keychain Tauri).

**Recomendación:** después del review panel; encadena naturalmente con diff commenting.

---

### 2.3 Terminal integrado
**Fuente:** OpenChamber  
**Complejidad:** 🟡 alta  
**Estado nuestro:** solo output ANSI en tool cards

- PTY por directorio, tabs, rendimiento con output pesado.
- Tauri: plugin PTY o sidecar; mucho trabajo de UI.

**Recomendación:** media-alta prioridad para paridad con OpenChamber.

---

### 2.4 Timeline de chat branchable (undo / redo / fork)
**Fuente:** OpenChamber  
**Complejidad:** 🟡 alta · depende de agente  
**Estado nuestro:** historial lineal

- `/undo`, `/redo`, fork desde cualquier turno.
- Circulo Electron: `Cmd+Z` revierte último turno incluyendo archivos.
- **Requiere** soporte del backend (OpenCode server / extensiones ACP); no es solo UI.

**Recomendación:** investigar si `opencode acp` expone revert; si no, feature flag “cuando exista API”.

---

### 2.5 Multi-agente en un prompt (comparación paralela)
**Fuente:** OpenChamber  
**Complejidad:** 🟡 alta  
**Estado nuestro:** un agente (`opencode acp`)

- Varios modelos/agentes con worktrees aislados.
- Depende de 1.2 + registry multi-agente.

**Recomendación:** roadmap largo; empezar por selector multi-agente simple (Cline, etc.).

---

### 2.6 Gestión MCP en UI (install templates, OAuth, toggles)
**Fuente:** soycanopa/circulo  
**Complejidad:** 🟡 media-alta  
**Estado nuestro:** MCP solo vía config OpenCode; bloqueamos design MCPs en Rust

- 19 plantillas one-click, pestañas Installed/Discover, skills como slash-commands.
- Con ACP: leer/escribir config OpenCode desde Tauri; no necesariamente SDK HTTP.

**Recomendación:** settings v2 — gran valor para usuarios power.

---

### 2.7 Browser de archivos + editor inline
**Fuente:** OpenChamber  
**Complejidad:** 🟡 media-alta  
**Estado nuestro:** `@` mentions y diffs; sin explorador

- Árbol de workspace, edición con syntax highlight, vim mode, preview markdown.
- Encaja con Tauri + permisos scoped.

**Recomendación:** después del review panel; puede compartir componentes Shiki/diff.

---

## Nivel 3 — Experiencia de chat y agente

### 3.1 Sub-agent cards (tareas delegadas)
**Fuente:** soycanopa/circulo  
**Complejidad:** 🟢 media · 🟡 si ACP no modela sub-sesiones  
**Estado nuestro:** tool calls planos

- Tarjetas colapsables con vista de sesión hija y progreso en vivo.

---

### 3.2 Preguntas interactivas del agente
**Fuente:** soycanopa/circulo  
**Complejidad:** 🟢 media  
**Estado nuestro:** solo permisos binarios

- Radio, checkbox, texto libre con atajos de teclado.
- Verificar eventos ACP `session/request_question` o equivalente OpenCode.

---

### 3.3 Adjuntos (imágenes, PDF)
**Fuente:** soycanopa/circulo  
**Complejidad:** 🟡 media  
**Estado nuestro:** no soportado

- Drag-and-drop al composer; warning si modelo no soporta visión.
- ACP: extender `send_prompt` con blobs en Rust.

---

### 3.4 Slash commands + skills en el input
**Fuente:** soycanopa/circulo  
**Complejidad:** 🟢 media  
**Estado nuestro:** no hay `/`

- Filtros Skills / Globales / Locales / CMD / MCP, chips, iconos tipo VS Code.
- Podemos exponer skills locales como comandos sin servidor Electron.

---

### 3.5 Compactación de sesión (manual / auto)
**Fuente:** soycanopa/circulo, OpenChamber  
**Complejidad:** 🟡 media  
**Estado nuestro:** no existe

- `/compact` y umbral automático de tokens.
- Depende de comando del agente; portar UI + invoke cuando API exista.

---

### 3.6 Comentarios inline en diffs → chat
**Fuente:** ambos  
**Complejidad:** 🟢 media-baja  
**Estado nuestro:** **parcial** (plan comment)

- Generalizar `planCommentMode` a comentarios anclados en líneas del review panel.

---

### 3.7 Sesiones desde GitHub Issues / PRs
**Fuente:** OpenChamber  
**Complejidad:** 🟡 media  
**Estado nuestro:** no existe

- Prefill de contexto desde issue/PR URL.
- Rust: Octokit + plantilla de prompt.

---

### 3.8 Visibilidad de contexto (tokens, coste, raw messages)
**Fuente:** OpenChamber  
**Complejidad:** 🟢 media-baja  
**Estado nuestro:** no existe

- Panel debug para power users; leer metadata de sesión ACP si está disponible.

---

### 3.9 Voice mode (entrada y lectura)
**Fuente:** OpenChamber  
**Complejidad:** 🟡 media  
**Estado nuestro:** no existe

- Web Speech API en webview Tauri; TTS para respuestas.

---

## Nivel 4 — Desktop, settings y polish

### 4.1 Panel Settings completo
**Fuente:** ambos  
**Complejidad:** 🟢 media-baja  
**Estado nuestro:** botón sin handler

- General, agente, MCP, skills, apariencia, atajos, about.
- **Prioridad inmediata** — deuda visible en la UI actual.

---

### 4.2 Command palette (`⌘K`)
**Fuente:** soycanopa/circulo  
**Complejidad:** 🟢 media-baja  
**Estado nuestro:** no existe

- Buscar sesiones, proyectos, toggles, comandos.
- Librería `cmdk` (ya usada en Circulo Electron).

---

### 4.3 Persistencia de drafts al cambiar sesión
**Fuente:** soycanopa/circulo  
**Complejidad:** 🟢 baja  
**Estado nuestro:** no guardamos draft

- Mapa `sessionId → draft` en localStorage o SQLite.

---

### 4.4 Paginación lazy del historial de chat
**Fuente:** soycanopa/circulo  
**Complejidad:** 🟢 baja  
**Estado nuestro:** cargamos todo en memoria

- Útil para sesiones largas.

---

### 4.5 Permisos “allow always”
**Fuente:** soycanopa/circulo  
**Complejidad:** 🟢 baja  
**Estado nuestro:** once / deny

- Extender `PermissionCard` + persistir reglas por tool/path.

---

### 4.6 Abrir en editor externo
**Fuente:** ambos  
**Complejidad:** 🟢 baja  
**Estado nuestro:** no existe

- VS Code, Cursor, Terminal vía `tauri-plugin-opener` + detección de apps.

---

### 4.7 System tray, badges y notificaciones nativas
**Fuente:** ambos  
**Complejidad:** 🟢 baja-media  
**Estado nuestro:** no existe

- Badge en dock cuando `awaiting_permission` o turno completado.

---

### 4.8 Auto-update + pipeline release
**Fuente:** soycanopa/circulo (Changesets, CI, electron-builder)  
**Complejidad:** 🟢 media · adaptar a Tauri  
**Estado nuestro:** builds manuales

- GitHub Actions + `tauri-action` + firmado/notarización macOS.

---

### 4.9 mDNS descubrimiento de servidores OpenCode
**Fuente:** soycanopa/circulo  
**Complejidad:** 🟢 baja · baja prioridad con ACP  
**Estado nuestro:** no aplica a stdio local

- Solo relevante si volvemos a soportar servidor remoto.

---

### 4.10 RTK token optimization toggle
**Fuente:** soycanopa/circulo  
**Complejidad:** 🟢 baja  
**Estado nuestro:** no existe

- Toggle en settings que pasa flag a config OpenCode.

---

## Nivel 5 — Quick wins (más sencillas)

### 5.1 Mermaid en markdown del chat
**Fuente:** OpenChamber  
**Complejidad:** 🟢 baja  
**Estado nuestro:** GFM sin mermaid

- Plugin `remark-mermaid` o render post-procesado.

---

### 5.2 Modo shell con `!` en el input
**Fuente:** OpenChamber  
**Complejidad:** 🟢 baja  
**Estado nuestro:** no existe

- Prefijo `!` ejecuta comando local scoped al proyecto (con confirmación).

---

### 5.3 Compartir mensaje como imagen
**Fuente:** OpenChamber  
**Complejidad:** 🟢 baja  
**Estado nuestro:** no existe

- `html-to-image` del bubble seleccionado.

---

### 5.4 Smart collapse en diffs (lockfiles, archivos gigantes)
**Fuente:** soycanopa/circulo  
**Complejidad:** 🟢 baja  
**Estado nuestro:** mostramos todo

- Heurísticas por path/tamaño en `inline-diff-block` y futuro review panel.

---

### 5.5 Iconos de tipo de archivo en listas
**Fuente:** OpenChamber  
**Complejidad:** 🟢 baja  
**Estado nuestro:** parcial

- Set VS Code-style en sidebar, `@` picker y tool cards.

---

### 5.6 Acento del sistema (macOS/Windows)
**Fuente:** soycanopa/circulo  
**Complejidad:** 🟢 baja  
**Estado nuestro:** paleta fija

- Leer accent color nativo y mapear a `--ring` / links.

---

### 5.7 Multi-agent registry (selector Cline, etc.)
**Fuente:** README nuestro  
**Complejidad:** 🟢 media-baja (solo OpenCode hoy)  
**Estado nuestro:** hardcoded `opencode acp`

- `agents/mod.rs` registry + UI; cada agente es un comando ACP distinto.

---

### 5.8 CI + tests
**Fuente:** ambos  
**Complejidad:** 🟢 baja-media  
**Estado nuestro:** sin CI

- `bun run check-types`, `cargo test`, lint en GitHub Actions.

---

### 5.9 Branding e iconos Circulo
**Fuente:** roadmap interno  
**Complejidad:** 🟢 baja  
**Estado nuestro:** iconos Tauri genéricos

- Reutilizar assets de `soycanopa/circulo/apps/desktop/resources/brand/`.

---

### 5.10 General Chat path portable
**Fuente:** deuda técnica nuestra  
**Complejidad:** 🟢 muy baja  
**Estado nuestro:** hardcoded `/Users/soycanopa`

- Configurar en settings o usar `~/` por defecto.

---

## Matriz resumen — Top 10 recomendados para Circulo ACP

| # | Feature | Origen | Esfuerzo | Impacto |
|---|---------|--------|----------|---------|
| 1 | Settings panel | Ambos | Medio-bajo | Alto |
| 2 | Review panel + diff comments | Circulo E. + OC | Alto | Muy alto |
| 3 | Command palette | Circulo E. | Bajo | Alto |
| 4 | Wizard migración (`configconv`) | Circulo E. | Alto | Alto (onboarding) |
| 5 | Slash commands / skills UI | Circulo E. | Medio | Alto |
| 6 | Permisos allow-always + drafts | Circulo E. | Bajo | Medio |
| 7 | Git commit/push (sin PR aún) | Ambos | Medio | Alto |
| 8 | Registry multi-agente | Nuestro | Medio-bajo | Alto |
| 9 | MCP settings UI | Circulo E. | Medio-alto | Medio-alto |
| 10 | CI + auto-update Tauri | Circulo E. | Medio | Alto (distribución) |

---

## Qué **no** portar tal cual

1. **Monorepo Turborepo + Electron main/preload** — ya migraste a Tauri; solo portar paquetes aislados (`configconv`, assets).
2. **OpenCode SDK SSE en el renderer** — rompe el modelo de seguridad ACP en Rust.
3. **PWA / túneles / VS Code extension** — productos distintos; no diluir foco desktop ACP.
4. **Servidor Hono embebido** — innecesario con Tauri invoke.

---

## Fases sugeridas

### Fase A — Cerrar MVP desktop (4–6 semanas)
- Settings, command palette, drafts, allow-always, branding, CI, path portable.
- Review panel v1 (lista de archivos tocados + diff simple).

### Fase B — Paridad Palot/OpenChamber (2–3 meses)
- Review panel v2 virtualizado, diff commenting, git commit/push.
- Slash commands, sub-agent cards, adjuntos, compact.
- Wizard migración v1.

### Fase C — Diferenciación (3+ meses)
- Automations, worktrees, MCP UI completa, terminal integrado.
- Undo/fork timeline si API agente lo permite.

---

*Última actualización: julio 2026 — basado en Circulo Tauri v0.1, soycanopa/circulo v0.15.x, openchamber main.*