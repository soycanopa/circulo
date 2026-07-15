# Circulo — Roadmap de producto

Documento de referencia para decidir qué traer de repos relacionados al **Circulo actual** (Tauri v2 + Rust + ACP stdio + React 19).

**Fuentes analizadas:**

| Repo | Stack | Relación con nosotros |
|------|-------|------------------------|
| [soycanopa/circulo](https://github.com/soycanopa/circulo) | Electron + OpenCode SDK (HTTP/SSE) + monorepo Turborepo | Fork previo tuyo de Palot; misma marca, distinta arquitectura |
| [openchamber/openchamber](https://github.com/openchamber/openchamber) | Desktop + Web/PWA + VS Code ext; OpenCode server | Referente de UX madura; 2k+ commits, comunidad activa |
| [Emanuele-web04/synara](https://github.com/Emanuele-web04/synara) | Electron + Bun server WebSocket + multi-proveedor directo (Codex app-server, etc.) + SQLite | Competidor cercano en visión “workspace local”; ~2.2k commits, v0.5.4, 1.3k★ |
| [craft-ai-agents/craft-agents-oss](https://github.com/craft-ai-agents/craft-agents-oss) | Electron + Claude/Pi SDK + `@craft-agent/ui` (Shiki, @pierre/diffs, TipTap) | **Referente #1 de polish en chat** — previews, diffs, planes, credenciales; ~97 commits, Apache 2.0 |

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
**Fuente:** OpenChamber, Synara, soycanopa/circulo  
**Complejidad:** 🔴 muy alta  
**Estado nuestro:** no existe

- OpenChamber: multi-agent desde un prompt con worktrees; merge con resolución de conflictos.
- **Synara:** `handoffThread` en `GitManager` crea/reutiliza worktrees, lleva cambios sin commitear (stash), y asocia threads a ramas aisladas; `worktreeHandoff.ts` modela intents `create-new` / `reuse-associated`.
- Circulo Electron: página de settings de worktrees para automations/experimentos.
- Requiere: gestión git en Rust (`git2` o comandos), UI de worktrees, política de cwd por sesión ACP.

**Recomendación:** después del review panel y git básico. **Synara** es la referencia más concreta para handoff git + worktree (aunque su motor vive en server Node, la lógica de `packages/shared/src/worktreeHandoff.ts` es portable).

---

### 1.3 Cliente dual ACP + OpenCode SDK (HTTP) / multi-proveedor directo
**Fuente:** implícito al comparar los cuatro proyectos  
**Complejidad:** 🔴 muy alta  
**Estado nuestro:** solo ACP

- soycanopa/circulo y OpenChamber asumen **servidor OpenCode** con SDK rico (undo, compact, attachments, MCP admin, etc.).
- **Synara** evita ACP y habla **directo** con cada proveedor (Codex app-server JSON-RPC, Claude Agent, Gemini, Grok, Pi, OpenCode, Cursor, Kilo) vía `apps/server` + WebSocket. `packages/contracts` define discovery, capabilities y eventos por proveedor.
- Muchas features “fáciles” en esos repos **no existen en ACP** o están incompletas.

**Recomendación:** **no** hacer dual stack a corto plazo. Priorizar gaps vía ACP + contribuir al protocolo. Para multi-proveedor, preferir **varios binarios ACP** (`opencode acp`, futuro `cline acp`, etc.) antes de replicar el registry directo de Synara. Reevaluar solo si ACP no cubre undo/attachments en 6 meses.

---

### 1.4 Acceso remoto
**Fuente:** OpenChamber (completo), Synara (mínimo viable)  
**Complejidad:** 🔴 alta (OC) · 🟡 media (Synara)  
**Estado nuestro:** app desktop local únicamente

- **OpenChamber:** Cloudflare tunnel, QR onboarding, `openchamber://` deep links, SSH port forwarding, host switcher.
- **Synara:** servidor HTTP/WebSocket con `--host`, `--port`, `--auth-token`; acceso LAN o Tailscale sin cloud propio ([REMOTE.md](https://github.com/Emanuele-web04/synara/blob/main/REMOTE.md)). Más simple que túneles gestionados.
- Contradice parcialmente la apuesta Tauri ligera + ACP local.

**Recomendación:** fuera de scope v1. Si algún día hacemos remoto, el modelo Synara (bind + token) es más alineado con Tauri que el de OpenChamber. Spin-off “Circulo Remote” mucho más adelante.

---

### 1.5 Handoff entre proveedores (cambiar modelo/agente con contexto)
**Fuente:** Synara  
**Complejidad:** 🔴 alta · 🟡 parcial con ACP  
**Estado nuestro:** un agente fijo por sesión

- Synara: `ThreadHandoff` persiste `sourceThreadId`, `sourceProvider`, mensajes importados y `bootstrapStatus`; el usuario puede pasar un hilo a otro proveedor manteniendo historial.
- Requiere: export/import de transcript, spawn de sesión nueva en otro backend, y normalización de mensajes entre formatos distintos.

**Recomendación:** roadmap largo. Atajo intermedio 🟡: “nueva sesión con resumen” o pegar transcript al cambiar de agente ACP; handoff completo solo si varios agentes ACP maduran.

---

### 1.6 Persistencia SQLite local (proyectos, threads, drafts)
**Fuente:** Synara (`state.sqlite`, proyecciones Effect)  
**Complejidad:** 🟡 media-alta  
**Estado nuestro:** localStorage + memoria en Rust

- Synara: server-side SQLite con migraciones, proyecciones de orchestration, drafts en `composerDraftStore`, retención de threads.
- En Tauri: `rusqlite` o `tauri-plugin-sql`; migrar keys `circulo-*` gradualmente.

**Recomendación:** fase B — antes de split views y cola de mensajes persistente. Evita límites de localStorage en sesiones largas.

---

### 1.7 Wizard de migración Claude Code / Cursor
**Fuente:** soycanopa/circulo (`packages/configconv`)  
**Complejidad:** 🟡 alta · 📦 port parcial  
**Estado nuestro:** no existe

- Convierte MCP, agents, commands, rules (`CLAUDE.md` → `AGENTS.md`), hooks e historial (`state.vscdb`).
- El paquete `configconv` es **reutilizable** independiente del shell Electron.

**Recomendación:** portar `configconv` como crate CLI o invocación Tauri; wizard en React. Alto impacto onboarding.

---

## Nivel 2 — Flujos de producto grandes

### 2.1 Layout split: chats paralelos + panel derecho (diff / terminal / browser)
**Fuente:** Synara  
**Complejidad:** 🟡 alta  
**Estado nuestro:** una vista chat a la vez

- Synara: `splitViewStore` — árbol de panes hasta 2×2, cada leaf con `threadId` + `ChatRightPanel` (diff, terminal, browser preview); drag-and-drop entre threads.
- Terminal drawer con toggle `mod+j`; browser preview embebido (`BrowserPanel.tsx`); dev servers por proyecto.

**Recomendación:** fase B — después de review panel v1. Es el diferenciador visual más fuerte de Synara vs chat plano. En Tauri: paneles web + PTY plugin; no requiere cambiar ACP.

---

### 2.2 Review panel dedicado (diffs de sesión)
**Fuente:** soycanopa/circulo (principal), OpenChamber, Synara  
**Complejidad:** 🟡 alta  
**Estado nuestro:** diffs inline en chat; sin panel lateral

- Virtualización (TanStack Virtual), worker pool para Shiki, smart collapse de lockfiles/archivos enormes.
- Synara integra el diff en el panel derecho del split view, con `diffTurnId` y `diffFilePath` por pane.
- Comentarios por línea → inyectar feedback al chat (nosotros ya tenemos patrón similar en **plan comment**).

**Recomendación:** **prioridad alta** — alinea con visión Palot; reutilizar `@pierre/diffs` + virtualización.

---

### 2.3 Git integrado: commit, push, PR
**Fuente:** OpenChamber, soycanopa/circulo, **Synara** (muy maduro)  
**Complejidad:** 🟡 alta  
**Estado nuestro:** no existe

- Diálogo commit/branch/push/PR GitHub desde la app.
- **Synara v0.5.4:** workspace nativo de Pull Requests vía `gh` CLI — listado cross-project, filtros, pin, merge/close/reopen, diffs y comentarios in-app (`pullRequests.logic.ts`, capa `git/` en server).
- Rust: `git` subprocess + API GitHub o `gh` (token en keychain Tauri).

**Recomendación:** después del review panel; encadena naturalmente con diff commenting. Para PRs, copiar el enfoque Synara (`gh` subprocess) es más rápido que API REST pura.

---

### 2.4 Terminal integrado
**Fuente:** OpenChamber, **Synara**  
**Complejidad:** 🟡 alta  
**Estado nuestro:** solo output ANSI en tool cards

- PTY por directorio, tabs, split (`mod+d`), new/close (`mod+n` / `mod+w`), toggle drawer (`mod+j`).
- Synara: terminal-first threads (`chat.newTerminal`), contexto de terminal adjunto al composer (`terminalContexts` en drafts).
- Tauri: plugin PTY o sidecar; mucho trabajo de UI.

**Recomendación:** media-alta prioridad; sube de prioridad si adoptamos split layout (2.1).

---

### 2.5 Timeline de chat branchable (undo / redo / fork)
**Fuente:** OpenChamber  
**Complejidad:** 🟡 alta · depende de agente  
**Estado nuestro:** historial lineal

- `/undo`, `/redo`, fork desde cualquier turno.
- Circulo Electron: `Cmd+Z` revierte último turno incluyendo archivos.
- **Requiere** soporte del backend (OpenCode server / extensiones ACP); no es solo UI.

**Recomendación:** investigar si `opencode acp` expone revert; si no, feature flag “cuando exista API”.

---

### 2.6 Multi-agente en un prompt (comparación paralela)
**Fuente:** OpenChamber  
**Complejidad:** 🟡 alta  
**Estado nuestro:** un agente (`opencode acp`)

- Varios modelos/agentes con worktrees aislados.
- Depende de 1.2 + registry multi-agente.

**Recomendación:** roadmap largo; empezar por selector multi-agente simple (Cline, etc.).

---

### 2.7 Gestión MCP en UI (install templates, OAuth, toggles)
**Fuente:** soycanopa/circulo  
**Complejidad:** 🟡 media-alta  
**Estado nuestro:** MCP solo vía config OpenCode; bloqueamos design MCPs en Rust

- 19 plantillas one-click, pestañas Installed/Discover, skills como slash-commands.
- Con ACP: leer/escribir config OpenCode desde Tauri; no necesariamente SDK HTTP.

**Recomendación:** settings v2 — gran valor para usuarios power.

---

### 2.8 Browser de archivos + editor inline
**Fuente:** OpenChamber  
**Complejidad:** 🟡 media-alta  
**Estado nuestro:** `@` mentions y diffs; sin explorador

- Árbol de workspace, edición con syntax highlight, vim mode, preview markdown.
- Encaja con Tauri + permisos scoped.

**Recomendación:** después del review panel; puede compartir componentes Shiki/diff.

---

## Nivel 2.9 — Chat polish (referencia Craft Agents)

Análisis profundo de [craft-agents-oss](https://github.com/craft-ai-agents/craft-agents-oss) centrado en **cómo se ve y se siente el chat**. Circulo ya tiene los esqueletos (`markdown-content`, `plan-preview-card`, `permission-card`, `inline-diff-block`, `tool-call-card`); Craft está 2–3 capas más adelante en normalización y presentación.

**Stack Craft relevante:** `packages/ui` — `react-markdown` + remark-gfm/math + Shiki 3 (cache LRU) + **@pierre/diffs** + overlays fullscreen. El chat agrupa mensajes en **turnos** (`TurnCard` + `turn-utils.ts`), no en burbujas planas.

### C1. Diffs reales con @pierre/diffs (inline + markdown + overlay)
**Fuente:** Craft (`MarkdownDiffBlock.tsx`, `diff-normalize.ts`, `ShikiDiffViewer.tsx`, `MultiDiffPreviewOverlay.tsx`)  
**Complejidad:** 🟢 media · **ACP-ready** · 📦 dependencia ya en `package.json`  
**Estado nuestro:** `inline-diff-block.tsx` hace diff línea-a-línea naive; `@pierre/diffs` declarado pero **sin usar**

- Fences ` ```diff ` en markdown → `PatchDiff` con error boundary → fallback a `CodeBlock`.
- `ensureUnifiedDiffFormat()` normaliza hunks sin headers `---/+++` (común en output de LLM).
- Overlays: diff single-file y multi-file estilo PR; word-level diff (`lineDiffType: 'word'`).
- `tool-parsers.ts` unifica parseo de resultados Edit/Write/Bash → overlay correcto.

**Recomendación:** **prioridad máxima en chat** — reemplazar `inline-diff-block` y diffs en `tool-call-card`. ROI inmediato; mismo paquete que Synara/Craft.

---

### C2. CodeBlock Shiki con cache, copiar y tema
**Fuente:** Craft (`CodeBlock.tsx`, `ShikiThemeContext.tsx`)  
**Complejidad:** 🟢 baja · **ACP-ready**  
**Estado nuestro:** `markdown-content.tsx` — Shiki sin cache, sin copy, tema fijo `github-dark-default`

- Cache LRU (200 entradas), aliases de lenguaje (`js`→`javascript`), 3 modos (`terminal`/`minimal`/`full`).
- Botón copiar; tema vía contexto (dark-only themes funcionan aunque el OS esté en light).

**Recomendación:** fase A — refactor de `markdown-content.tsx` antes de fences especializados.

---

### C3. Permisos legibles en el composer (no JSON crudo)
**Fuente:** Craft (`PermissionRequest.tsx`, `CompactPermissionModeSelector.tsx`)  
**Complejidad:** 🟢 media-baja · **ACP-ready**  
**Estado nuestro:** `permission-card.tsx` muestra `JSON.stringify(toolCall)` + Aprobar/Denegar

- Craft: badge de tool, descripción humana, preview del comando (scrollable), **Allow / Always Allow / Deny**.
- El prompt vive en el **área de input** (reemplaza composer), no como tarjeta flotante sobre mensajes.
- Modos de sesión separados: safe / ask / allow-all (concepto del agente Craft; en ACP mapear a `allow_once` / `allow_always` / `reject_once`).

**Recomendación:** **prioridad alta** — enriquecer `permission-card` o moverla al composer como Craft. Circulo ya tiene `allow_always` en roadmap (4.6); unificar aquí.

---

### C4. Credenciales y datos sensibles
**Fuente:** Craft (`CredentialRequest.tsx`, `AuthRequestCard.tsx`, `credential-prompt.ts`)  
**Complejidad:** 🟡 media · depende de ACP/agente  
**Estado nuestro:** no existe

- Modos: `bearer`, `basic`, `header`, `multi-header` (varios API keys a la vez).
- Form en composer con show/hide password, validación, Escape=cancel; compatible 1Password (`action=sourceUrl`).
- Historial en transcript como turno `auth-request` separado (no mezclado con respuesta del asistente).
- Backend: keychain seguro; nunca persistir secretos en DOM/logs.

**Recomendación:** preparar UI ahora (`CredentialPrompt.tsx`); cablear cuando ACP/OpenCode exponga `session/request_credential` o equivalente. Diseño 100% portable.

---

### C5. Plan preview — Accept & Compact + integración en turno
**Fuente:** Craft (`AcceptPlanDropdown.tsx`, `TurnCard` variante plan, `submit-plan.ts`)  
**Complejidad:** 🟢 baja-media · **ACP-ready**  
**Estado nuestro:** `plan-preview-card.tsx` — Aceptar / Comentar / Rechazar / Descargar (buena base)

- Craft añade **“Accept & Compact”**: resume conversación antes de ejecutar (útil con contexto largo).
- Plan embebido en el turno del asistente (header verde, scroll con fade), no tarjeta flotante aislada.
- Fullscreen `DocumentFormattedMarkdownOverlay` para planes largos; anotaciones inline (TipTap — no portar entero).

**Recomendación:** añadir dropdown “Aceptar y compactar” en `plan-preview-card`; mantener tarjeta separada (ya funciona con ACP). Compact vía prompt `/compact` o comando del agente.

---

### C6. Markdown rico (fences especializados)
**Fuente:** Craft (`Markdown.tsx` — router de fences)  
**Complejidad:** 🟡 media (incremental)  
**Estado nuestro:** GFM + code blocks básicos

- Fences custom: `diff`, `json`, `mermaid`, `datatable`, `html-preview`, `pdf-preview`, `image-preview`, `latex`.
- `MemoizedMarkdown` por `id` para no re-parsear en cada delta de streaming.
- `CollapsibleMarkdownContext` — headings colapsables en docs largos.
- Buffering de stream en `ResponseCard` (40 palabras / 500ms–2.5s) para no mostrar ruido mientras “piensa”.

**Recomendación:** incremental — primero `diff` (C1) y `json`; luego mermaid (5.1). Evaluar buffering adaptado a expectativa de streaming ACP (no copiar literal).

---

### C7. Tool calls como “activity trace” en el turno
**Fuente:** Craft (`TurnCard.tsx`, `turn-utils.ts`, `deriveTurnPhase()`)  
**Complejidad:** 🟡 media-alta  
**Estado nuestro:** `tool-call-card/group` — collapsible, ANSI en execute

- Tools como líneas de actividad dentro del turno (icono + nombre + estado), no tarjetas sueltas por mensaje.
- Fases: `pending` → `tool_active` → `awaiting` → `streaming` → `complete` — evita “thinking forever” entre tools.
- `ActivityGroupRow` para subagentes Task; botón multi-diff si hay varios Edit/Write.
- Modo `informative` vs `detailed` (preferencia de usuario).

**Recomendación:** no portar `TurnCard` entero (ACP entrega mensajes planos). Sí portar **`deriveTurnPhase()`** ligero sobre `toolCalls` del mensaje actual + thinking indicator inteligente.

---

### C8. Overlays fullscreen para expandir tool output
**Fuente:** Craft (`CodePreviewOverlay.tsx`, `MultiDiffPreviewOverlay.tsx`, `ActivityCardsOverlay.tsx`)  
**Complejidad:** 🟡 media  
**Estado nuestro:** expandir tool muestra `<pre>` plano

- Click en actividad Read/Write → overlay con Shiki, rango de líneas, badge Read/Write.
- Multi-diff consolidado por path; tabs para resultados MCP/browser heterogéneos.

**Recomendación:** fase B — modal/dialog Tauri-friendly; depende de C1 + C2.

---

## Nivel 3 — Experiencia de chat y agente

### 3.1 Cola de mensajes en el composer
**Fuente:** Synara  
**Complejidad:** 🟢 media-baja · **ACP-ready**  
**Estado nuestro:** no se puede enviar mientras el agente trabaja

- Synara: `QueuedComposerTurn` en `composerDraftStore` — turnos `chat` y `plan-follow-up` con provider/model/mode/adjuntos preservados; drenado automático al terminar el turno activo.
- UX: header flotante sobre el composer con preview del siguiente mensaje; ya marcado ✅ en su `TODO.md`.

**Recomendación:** **quick win de alto impacto** — solo UI + buffer en frontend; al completar `acp:prompt_complete`, enviar el siguiente `send_prompt`. Encaja perfectamente con ACP.

---

### 3.2 Sub-agent cards (tareas delegadas)
**Fuente:** soycanopa/circulo, Craft (`ActivityGroupRow`)  
**Complejidad:** 🟢 media · 🟡 si ACP no modela sub-sesiones  
**Estado nuestro:** tool calls planos

- Tarjetas colapsables con vista de sesión hija y progreso en vivo.
- Craft anida tools por `parentId`/`depth` dentro del turno.

---

### 3.3 Preguntas interactivas del agente
**Fuente:** soycanopa/circulo, Craft (`StructuredInput.tsx`)  
**Complejidad:** 🟢 media  
**Estado nuestro:** solo permisos binarios

- Radio, checkbox, texto libre con atajos de teclado.
- Craft unifica permission | credential | admin_approval en un slot estructurado del composer.
- Verificar eventos ACP `session/request_question` o equivalente OpenCode.

---

### 3.4 Adjuntos (imágenes, PDF)
**Fuente:** soycanopa/circulo  
**Complejidad:** 🟡 media  
**Estado nuestro:** no soportado

- Drag-and-drop al composer; warning si modelo no soporta visión.
- ACP: extender `send_prompt` con blobs en Rust.

---

### 3.5 Slash commands + skills en el input
**Fuente:** soycanopa/circulo, Synara  
**Complejidad:** 🟢 media  
**Estado nuestro:** no hay `/`

- Filtros Skills / Globales / Locales / CMD / MCP, chips, iconos tipo VS Code.
- Synara: `ProviderComposerCapabilities` por proveedor (`supportsSkillDiscovery`, `supportsNativeSlashCommandDiscovery`, etc.) en `packages/contracts`.
- Podemos exponer skills locales como comandos sin servidor Electron; capabilities matrix útil cuando tengamos multi-agente ACP.

---

### 3.6 Compactación de sesión (manual / auto)
**Fuente:** soycanopa/circulo, OpenChamber  
**Complejidad:** 🟡 media  
**Estado nuestro:** no existe

- `/compact` y umbral automático de tokens.
- Depende de comando del agente; portar UI + invoke cuando API exista.

---

### 3.7 Comentarios inline en diffs → chat
**Fuente:** ambos  
**Complejidad:** 🟢 media-baja  
**Estado nuestro:** **parcial** (plan comment)

- Generalizar `planCommentMode` a comentarios anclados en líneas del review panel.

---

### 3.8 Sesiones desde GitHub Issues / PRs
**Fuente:** OpenChamber  
**Complejidad:** 🟡 media  
**Estado nuestro:** no existe

- Prefill de contexto desde issue/PR URL.
- Rust: Octokit + plantilla de prompt.

---

### 3.9 Visibilidad de contexto (tokens, coste, raw messages)
**Fuente:** OpenChamber  
**Complejidad:** 🟢 media-baja  
**Estado nuestro:** no existe

- Panel debug para power users; leer metadata de sesión ACP si está disponible.

---

### 3.10 Transcript scroll inteligente (anti-jank)
**Fuente:** Synara (`AGENTS.md`, `chat-scroll.test.ts`, `MessagesTimeline.tsx`)  
**Complejidad:** 🟢 media-baja  
**Estado nuestro:** scroll básico

- Auto-stick solo con texto assistant real en streaming; tool rows y permisos pendientes no re-disparan scroll.
- Evitar feedback loops con virtualización (`measure()` desacoplado de stick).
- Synara documenta esto como guardrail obligatorio para agentes que toquen el chat.

**Recomendación:** aplicar al escalar listas largas; tests de regresión como en Synara.

---

### 3.11 Voice mode (entrada y lectura)
**Fuente:** OpenChamber  
**Complejidad:** 🟡 media  
**Estado nuestro:** no existe

- Web Speech API en webview Tauri; TTS para respuestas.

---

## Nivel 4 — Desktop, settings y polish

### 4.1 Panel Settings completo
**Fuente:** OpenChamber, soycanopa/circulo, Synara  
**Complejidad:** 🟢 media-baja  
**Estado nuestro:** botón sin handler

- General, agente, MCP, skills, apariencia, atajos, about.
- Synara: settings de proveedores, usage por cuenta, worktree defaults, AppSnap (captura macOS).
- **Prioridad inmediata** — deuda visible en la UI actual.

---

### 4.2 Keybindings configurables (JSON)
**Fuente:** Synara ([KEYBINDINGS.md](https://github.com/Emanuele-web04/synara/blob/main/KEYBINDINGS.md))  
**Complejidad:** 🟢 media-baja  
**Estado nuestro:** atajos hardcoded o inexistentes

- Archivo `~/.circulo/keybindings.json`: array `{ key, command, when? }`.
- Comandos: `terminal.toggle`, `chat.new`, `composer.focus.toggle`, `editor.openFavorite`, `script.{id}.run`.
- Condiciones `when`: `terminalFocus`, `terminalOpen` con `&&`, `||`, `!`.
- Schema en Rust o TS; defaults en código.

**Recomendación:** fase A — bajo esfuerzo, alto valor power-user; Synara tiene tests en `keybindings.test.ts` como referencia.

---

### 4.3 Command palette (`⌘K`)
**Fuente:** soycanopa/circulo  
**Complejidad:** 🟢 media-baja  
**Estado nuestro:** no existe

- Buscar sesiones, proyectos, toggles, comandos.
- Librería `cmdk` (ya usada en Circulo Electron).

---

### 4.4 Persistencia de drafts al cambiar sesión
**Fuente:** soycanopa/circulo  
**Complejidad:** 🟢 baja  
**Estado nuestro:** no guardamos draft

- Mapa `sessionId → draft` en localStorage o SQLite.

---

### 4.5 Paginación lazy del historial de chat
**Fuente:** soycanopa/circulo  
**Complejidad:** 🟢 baja  
**Estado nuestro:** cargamos todo en memoria

- Útil para sesiones largas.

---

### 4.6 Permisos “allow always”
**Fuente:** soycanopa/circulo, **Craft** (`PermissionRequest` — “Always Allow”)  
**Complejidad:** 🟢 baja  
**Estado nuestro:** once / deny

- Extender `PermissionCard` + persistir reglas por tool/path.
- Unificar con rediseño C3: tres botones visibles, tip “recuerda para la sesión”.

---

### 4.7 Abrir en editor externo
**Fuente:** ambos  
**Complejidad:** 🟢 baja  
**Estado nuestro:** no existe

- VS Code, Cursor, Terminal vía `tauri-plugin-opener` + detección de apps.

---

### 4.8 Animaciones disclosure unificadas
**Fuente:** Synara (`disclosureMotion.ts`, `DisclosureRegion.tsx`)  
**Complejidad:** 🟢 baja · 📦 patrón portable  
**Estado nuestro:** animaciones ad hoc en sidebar

- Un solo módulo: 220ms `ease-out`, `grid-template-rows` + opacity, chevron rotatorio, `motion-reduce` fallback.
- Obligatorio para sidebar expand/collapse, tool cards, review panel.

**Recomendación:** fase A polish — copiar el patrón (no el paquete Effect); evita inconsistencias visuales al crecer la UI.

---

### 4.9 Restauración de ventana (posición, tamaño, maximizado)
**Fuente:** Synara v0.5.4  
**Complejidad:** 🟢 baja  
**Estado nuestro:** Tauri defaults

- Persistir bounds entre sesiones; validar monitor al reabrir (evitar ventana off-screen).
- Tauri: `tauri-plugin-window-state` o store manual en Rust.

---

### 4.10 Retención / límite de threads en sidebar
**Fuente:** Synara (`threadRetention.ts`, sidebar “últimos 10”)  
**Complejidad:** 🟢 baja  
**Estado nuestro:** listamos todo

- Mostrar N threads recientes por proyecto; archivar/ocultar inactivos tras 7 días (configurable).
- Synara usa sweep server-side; nosotros podemos hacerlo en Rust o frontend con SQLite.

---

### 4.11 System tray, badges y notificaciones nativas
**Fuente:** ambos  
**Complejidad:** 🟢 baja-media  
**Estado nuestro:** no existe

- Badge en dock cuando `awaiting_permission` o turno completado.

---

### 4.12 Auto-update + pipeline release
**Fuente:** soycanopa/circulo, Synara (CI profundo)  
**Complejidad:** 🟢 media · adaptar a Tauri  
**Estado nuestro:** builds manuales

- GitHub Actions + `tauri-action` + firmado/notarización macOS.
- Synara: CI con fmt, lint, typecheck, **2670+ tests web**, browser tests, desktop smoke, release smoke — referencia de madurez.

---

### 4.13 mDNS descubrimiento de servidores OpenCode
**Fuente:** soycanopa/circulo  
**Complejidad:** 🟢 baja · baja prioridad con ACP  
**Estado nuestro:** no aplica a stdio local

- Solo relevante si volvemos a soportar servidor remoto.

---

### 4.14 RTK token optimization toggle
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
**Fuente:** soycanopa/circulo, Craft (`MultiDiffPreviewOverlay` — foco por archivo)  
**Complejidad:** 🟢 baja  
**Estado nuestro:** mostramos todo

- Heurísticas por path/tamaño en diffs Pierre y futuro review panel.

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
**Fuente:** README nuestro, Synara (`ProviderDiscoveryKind`: codex, claudeAgent, cursor, gemini, grok, opencode, pi, kilo, droid)  
**Complejidad:** 🟢 media-baja (solo OpenCode hoy)  
**Estado nuestro:** hardcoded `opencode acp`

- `agents/mod.rs` registry + UI; cada agente es un comando ACP distinto.
- Synara modela capabilities por proveedor; nosotros podemos mapear lo mismo a “qué expone cada binario ACP” sin replicar su server.

---

### 5.8 CI + tests
**Fuente:** OpenChamber, soycanopa/circulo, **Synara** (benchmark)  
**Complejidad:** 🟢 baja-media  
**Estado nuestro:** sin CI

- `bun run check-types`, `cargo test`, lint en GitHub Actions.
- Synara: tests de scroll, keybindings, handoff git, split views, provider discovery — buen modelo de qué testear en UI de agente.

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

## Matriz resumen — Top 15 recomendados para Circulo ACP

| # | Feature | Origen | Esfuerzo | Impacto | ACP |
|---|---------|--------|----------|---------|-----|
| 1 | **@pierre/diffs en chat** (C1) | **Craft** | Medio | **Muy alto** | 🟢 |
| 2 | **Permisos legibles** (C3) + allow-always | **Craft** + Circulo E. | Bajo-medio | Alto | 🟢 |
| 3 | **CodeBlock Shiki** cache+copy (C2) | **Craft** | Bajo | Alto | 🟢 |
| 4 | Settings panel | OC + Circulo E. + Synara | Medio-bajo | Alto | 🟢 |
| 5 | Review panel + diff comments | Circulo E. + OC + Craft C8 | Alto | Muy alto | 🟢 |
| 6 | Cola de mensajes en composer | Synara | Bajo | Alto | 🟢 |
| 7 | Fases de turno + thinking (C7 ligero) | Craft | Medio | Medio-alto | 🟢 |
| 8 | Plan: Aceptar y compactar (C5) | Craft | Bajo | Medio | 🟢 |
| 9 | Command palette | Circulo E. | Bajo | Alto | 🟢 |
| 10 | Keybindings JSON | Synara | Bajo | Medio-alto | 🟢 |
| 11 | Slash commands / skills UI | Circulo E. + Synara | Medio | Alto | 🟢 |
| 12 | Git commit/push (sin PR aún) | OC + Synara | Medio | Alto | 🟡 |
| 13 | Registry multi-agente ACP | Nuestro + Synara | Medio-bajo | Alto | 🟢 |
| 14 | Credential prompt UI (C4) | Craft | Medio | Medio (futuro) | 🟡 |
| 15 | CI + auto-update Tauri | Circulo E. + Synara | Medio | Alto (distribución) | 🟢 |

**Honorable mentions (fase B):** overlays tool fullscreen (C8), fences json/mermaid (C6), split layout Synara, SQLite local, wizard `configconv`.

---

## Qué **no** portar tal cual

1. **Monorepo Turborepo + Electron main/preload** — ya migraste a Tauri; solo portar paquetes aislados (`configconv`, assets).
2. **OpenCode SDK SSE en el renderer** — rompe el modelo de seguridad ACP en Rust.
3. **PWA / túneles Cloudflare / VS Code extension** — productos distintos; no diluir foco desktop ACP.
4. **Servidor Hono / WebSocket Synara** — Synara separa `apps/server` (orchestration, proyecciones, git) del UI; en Circulo eso vive en Rust + Tauri invoke, no en un server Node paralelo.
5. **Registry multi-proveedor directo (Codex app-server, etc.)** — alto mantenimiento; usar ACP como capa unificada.
6. **Effect-TS + `packages/contracts` completo** — excelente diseño en Synara, pero stack distinto; tomar *ideas* de schemas, no el runtime Effect.
7. **AppSnap / captura nativa macOS** — nice-to-have de Synara; no prioridad frente a chat/git/core.
8. **`TurnCard` completo + agrupación por turno Craft** — ACP entrega mensajes planos; portar solo fases ligeras (C7), no reescribir pipeline.
9. **Ecosistema TipTap** (editor, anotaciones, bubble menus) — Craft es document-centric; Circulo es visor de chat.
10. **Sistema de sources/credentials backend Craft** — `credential-manager`, OAuth multi-proveedor; distinto de ACP stdio.
11. **Buffering agresivo de stream (40 palabras)** — puede chocar con streaming inmediato ACP; adaptar umbrales.

---

## Fases sugeridas

### Fase A — Cerrar MVP desktop (4–6 semanas)
- **Chat polish Craft (bloque C1–C3):** @pierre/diffs, CodeBlock mejorado, permisos legibles + allow-always.
- Settings, command palette, **cola de mensajes**, **keybindings JSON**, drafts, branding, CI, path portable.
- **Disclosure motion** unificado, **window state** restoration.
- Plan: **Aceptar y compactar** (C5). Fases de turno ligeras (C7).
- Review panel v1 (lista de archivos tocados + diff Pierre).

### Fase B — Paridad Palot / OpenChamber / Synara / Craft (2–3 meses)
- Review panel v2 virtualizado, diff commenting, git commit/push; PR workspace vía `gh` (patrón Synara).
- **Overlays tool fullscreen** (C8), fences json/mermaid (C6), credential prompt cableado si ACP lo expone (C4).
- **Split layout** (chat + diff/terminal), terminal integrado, SQLite local.
- Slash commands, sub-agent cards, adjuntos, compact, transcript scroll guardrails.
- Wizard migración v1.

### Fase C — Diferenciación (3+ meses)
- Automations, worktrees + handoff git (Synara), MCP UI completa, browser preview.
- Provider handoff entre agentes ACP, undo/fork timeline si API agente lo permite.
- Acceso remoto LAN simple (opcional, modelo Synara).

---

---

## Synara — resumen de lo portable

| Idea Synara | Portable a Circulo ACP | Notas |
|-------------|------------------------|-------|
| Cola de mensajes | ✅ Sí | Frontend + `send_prompt` al completar turno |
| Keybindings JSON | ✅ Sí | `~/.circulo/keybindings.json` |
| Disclosure motion | ✅ Sí | Copiar patrón CSS/TS |
| Split chat + panel derecho | ✅ Sí (UI) | Sin cambiar backend |
| Worktree handoff git | 🟡 Parcial | Lógica en Rust; inspirarse en `worktreeHandoff.ts` |
| PR workspace (`gh`) | 🟡 Parcial | Subprocess desde Tauri |
| Provider handoff | 🟡 Parcial | Resumen al cambiar agente ACP; full import es difícil |
| Multi-proveedor directo | ❌ No | Usar varios binarios ACP |
| Server WebSocket + orchestration | ❌ No | Ya cubierto por Rust/Tauri |
| Remote LAN | 🟡 Más adelante | Más simple que OpenChamber |

---

## Craft Agents — resumen chat UX portable

| Área Craft | Archivos clave | Circulo hoy | Acción |
|------------|----------------|-------------|--------|
| Diffs markdown + tools | `MarkdownDiffBlock.tsx`, `diff-normalize.ts` | `inline-diff-block` naive | Activar `@pierre/diffs` (C1) |
| Code preview | `CodeBlock.tsx`, `ShikiThemeContext.tsx` | Shiki básico sin cache | Cache + copy + tema (C2) |
| Permisos | `PermissionRequest.tsx` | JSON crudo en card | Rediseñar composer (C3) |
| Credenciales | `CredentialRequest.tsx`, `AuthRequestCard.tsx` | No existe | UI preparada (C4) |
| Plan | `AcceptPlanDropdown.tsx`, plan en `TurnCard` | `plan-preview-card` OK | + Accept & Compact (C5) |
| Markdown rico | `Markdown.tsx` (10+ fences) | GFM básico | Incremental (C6) |
| Tool trace | `TurnCard`, `deriveTurnPhase()` | `tool-call-card` | Fases ligeras (C7) |
| Overlays | `*PreviewOverlay.tsx` | `<pre>` plano | Modal fase B (C8) |

**Mapa de archivos Craft:**
```
packages/ui/src/components/markdown/Markdown.tsx
packages/ui/src/components/markdown/CodeBlock.tsx
packages/ui/src/components/markdown/MarkdownDiffBlock.tsx
packages/ui/src/components/chat/TurnCard.tsx
packages/ui/src/components/chat/turn-utils.ts
packages/ui/src/components/chat/AcceptPlanDropdown.tsx
apps/electron/.../structured/PermissionRequest.tsx
apps/electron/.../structured/CredentialRequest.tsx
```

**Mapa Circulo (equivalentes a mejorar):**
```
src/components/chat/markdown-content.tsx      ← C2, C6
src/components/diff/inline-diff-block.tsx     ← C1
src/components/permissions/permission-card.tsx ← C3
src/components/chat/plan-preview-card.tsx     ← C5
src/components/tools/tool-call-card.tsx       ← C1, C7, C8
```

---

*Última actualización: julio 2026 — basado en Circulo Tauri v0.1, soycanopa/circulo v0.15.x, openchamber main, synara v0.5.4, craft-agents-oss main.*