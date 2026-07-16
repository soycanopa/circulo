# Skills recomendados para Circulo

Lista curada de [skills.sh](https://skills.sh/) útiles para desarrollar **Circulo** (Tauri v2 + Rust + React 19 + ACP + OpenCode).  
Criterios: **installs altos**, fuente confiable, y relevancia directa para desktop AI, UI nativa, o calidad de código.

> Instalar: `npx skills add <owner/repo@skill> -g -y`  
> Descubrir más: `npx skills find <query>`

---

## Tier 1 — Esenciales para el día a día

### 1. `find-skills`
- **Paquete:** `vercel-labs/skills@find-skills`
- **Installs:** ~2.5M
- **URL:** https://skills.sh/vercel-labs/skills/find-skills
- **Por qué:** Meta-skill para descubrir e instalar otras skills del ecosistema. Circulo va a crecer en features (review panel, MCP UI, git); este skill acelera encontrar capacidades sin buscar manualmente.

---

### 2. `vercel-react-best-practices`
- **Paquete:** `vercel-labs/agent-skills@vercel-react-best-practices`
- **Installs:** ~554K
- **URL:** https://skills.sh/vercel-labs/agent-skills/vercel-react-best-practices
- **Por qué:** Circulo es React 19 con streaming, listas largas, selectores en portal y sidebar animado. Guías de rendimiento de Vercel (memo, suspense, list virtualization patterns) evitan jank al escalar el chat y el review panel.

---

### 3. `web-design-guidelines`
- **Paquete:** `vercel-labs/agent-skills@web-design-guidelines`
- **Installs:** ~466K
- **URL:** https://skills.sh/vercel-labs/agent-skills/web-design-guidelines
- **Por qué:** Checklist de UX/UI (jerarquía, spacing, estados, accesibilidad). Útil para pulir frosted glass, selectores, plan preview y futuro settings sin perder coherencia Palot/ZCode.

---

### 4. `frontend-design`
- **Paquete:** `anthropics/skills@frontend-design`
- **Installs:** ~668K
- **URL:** https://skills.sh/anthropics/skills/frontend-design
- **Por qué:** Anthropic oficial; orientado a interfaces frontend con criterio estético y funcional. Complementa nuestro dark theme y ayuda al rebrand visual de Circulo.

---

### 5. `shadcn`
- **Paquete:** `shadcn/ui@shadcn`
- **Installs:** ~237K
- **URL:** https://skills.sh/shadcn/ui/shadcn
- **Por qué:** El Circulo Electron (`soycanopa/circulo`) ya usa shadcn/ui en `@circulo/ui`. Si portamos command palette, settings y dialogs, este skill alinea patrones de componentes con la versión anterior del producto.

---

## Tier 2 — Desktop nativo (Tauri / macOS)

### 6. `macos-patterns` *(instalado en el repo)*
- **Paquete:** `fayazara/macos-app-skills@macos-patterns`
- **Ruta:** `.agents/skills/macos-patterns/SKILL.md`
- **Por qué:** Circulo ya usa vibrancy, titlebar overlay, drag regions y frosted shell. Este skill evita anti-patrones web (z-index mental model, clipboard naive) al añadir tray, notificaciones, keychain y mini-windows.

---

### 7. `tauri-v2` *(instalado en el repo)*
- **Paquete:** `nodnarbnitram/claude-code-extensions@tauri-v2`
- **Ruta:** `.agents/skills/tauri-v2/SKILL.md`
- **Por qué:** Foco en Tauri v2 (nuestra versión). Útil para capabilities, plugins PTY, IPC y empaquetado multi-plataforma.

---

### 8. `rust-best-practices` *(instalado en el repo)*
- **Paquete:** `apollographql/skills@rust-best-practices`
- **Ruta:** `.agents/skills/rust-best-practices/SKILL.md`
- **Por qué:** Sustituto seguro para patrones Rust en desktop: async, módulos, testing. Cubre `acp/runner.rs` y futuro scheduler sin depender de skills marcados como maliciosos.

---

## Tier 3 — Calidad, arquitectura y roadmap

### 10. `improve-codebase-architecture`
- **Paquete:** `mattpocock/skills@improve-codebase-architecture`
- **Installs:** ~465K
- **URL:** https://skills.sh/mattpocock/skills/improve-codebase-architecture
- **Por qué:** Circulo crecerá desde MVP a review panel + settings + git. Este skill empuja decisiones de módulos, boundaries frontend/Rust y deuda controlada.

---

### 11. `vercel-composition-patterns`
- **Paquete:** `vercel-labs/agent-skills@vercel-composition-patterns`
- **Installs:** ~250K
- **URL:** https://skills.sh/vercel-labs/agent-skills/vercel-composition-patterns
- **Por qué:** Patrones de composición React (compound components, slots). Aplica a `SidebarLayout`, tool cards, plan preview y futuros sub-agent cards sin prop drilling.

---

### 12. `writing-plans` + `executing-plans`
- **Paquetes:**
  - `obra/superpowers@writing-plans` (~186K)
  - `obra/superpowers@executing-plans` (~155K)
- **URLs:** https://skills.sh/obra/superpowers/writing-plans · https://skills.sh/obra/superpowers/executing-plans
- **Por qué:** Circulo tiene `ROADMAP.md` extenso; estos skills estructuran planes en PRs ejecutables — ideal para fases A/B/C del roadmap.

---

### 13. `systematic-debugging`
- **Paquete:** `obra/superpowers@systematic-debugging`
- **Installs:** ~188K
- **URL:** https://skills.sh/obra/superpowers/systematic-debugging
- **Por qué:** Debugging ACP es difícil (stdio, eventos duplicados, permisos colgados). Metodología sistemática para `RUST_LOG`, bridge de eventos y repro de bugs de streaming.

---

### 14. `requesting-code-review` + `receiving-code-review`
- **Paquetes:**
  - `obra/superpowers@requesting-code-review` (~168K)
  - `obra/superpowers@receiving-code-review` (~139K)
- **Por qué:** Antes de publicar en GitHub, revisiones de seguridad (permisos ACP, path scoping `@`) y UI crítica (sidebar, plan mode).

---

## Tier 4 — Features futuras del producto

### 15. `mcp-builder`
- **Paquete:** `anthropics/skills@mcp-builder`
- **Installs:** ~90K
- **URL:** https://skills.sh/anthropics/skills/mcp-builder
- **Por qué:** Roadmap incluye MCP settings UI (como Circulo Electron). Skill oficial para diseñar/integrar servidores MCP compatible con OpenCode.

---

### 16. `emil-design-eng`
- **Paquete:** `emilkowalski/skills@emil-design-eng`
- **Installs:** ~140K
- **URL:** https://skills.sh/emilkowalski/skills/emil-design-eng
- **Por qué:** Design engineering (animaciones, micro-interacciones, perceived performance). Refuerza sidebar resize smooth, frosted glass y transiciones del review panel.

---

### 17. `impeccable` (familia)
- **Paquete base:** `pbakaus/impeccable` (~195K)
- **Sub-skills útiles:** `polish`, `critique`, `distill`
- **URL:** https://skills.sh/pbakaus/impeccable/impeccable
- **Por qué:** Pass de pulido visual en componentes desktop — spacing, contraste, densidad — antes de releases públicas.

---

### 18. `playwright-cli`
- **Paquete:** `microsoft/playwright-cli@playwright-cli`
- **Installs:** ~87K
- **URL:** https://skills.sh/microsoft/playwright-cli/playwright-cli
- **Por qué:** E2E del frontend en `bun run dev` (sin Tauri) y smoke tests post-build. Microsoft oficial; encaja con CI del roadmap.

---

### 19. `skill-creator`
- **Paquete:** `anthropics/skills@skill-creator`
- **Installs:** ~315K
- **URL:** https://skills.sh/anthropics/skills/skill-creator
- **Por qué:** Circulo Electron expone skills como slash-commands; podemos crear skills propias (`circulo-acp`, `circulo-tauri`) para el equipo y usuarios avanzados.

---

### 20. `using-git-worktrees`
- **Paquete:** `obra/superpowers@using-git-worktrees`
- **Installs:** ~136K
- **URL:** https://skills.sh/obra/superpowers/using-git-worktrees
- **Por qué:** Alineado con roadmap de worktrees aislados (OpenChamber / Circulo Electron). Guía desarrollo paralelo antes de implementar la feature en la app.

---

## Instalación rápida (bundle recomendado)

```bash
# Core web + diseño
npx skills add vercel-labs/skills@find-skills -g -y
npx skills add vercel-labs/agent-skills@vercel-react-best-practices -g -y
npx skills add vercel-labs/agent-skills@web-design-guidelines -g -y
npx skills add anthropics/skills@frontend-design -g -y
npx skills add shadcn/ui@shadcn -g -y

# Desktop (usar solo paquetes sin alerta de skills.sh)
npx skills add fayazara/macos-app-skills@macos-patterns -g -y
npx skills add nodnarbnitram/claude-code-extensions@tauri-v2 -g -y
npx skills add apollographql/skills@rust-best-practices -g -y

# Calidad y roadmap
npx skills add obra/superpowers@writing-plans -g -y
npx skills add obra/superpowers@systematic-debugging -g -y
npx skills add obra/superpowers@requesting-code-review -g -y
```

---

## Skills del repo Circulo Electron (ya presentes)

En [soycanopa/circulo](https://github.com/soycanopa/circulo) ya está instalado:

- `.agents/skills/react-best-practices` — equivalente cercano a `vercel-react-best-practices`

Si clonas o sincronizas ese repo, puedes reutilizar esa carpeta en el proyecto Tauri en lugar de duplicar.

---

## No recomendados para Circulo (ahora)

| Skill / categoría | Motivo |
|-------------------|--------|
| `martinholovsky/claude-skills-generator@tauri` | skills.sh lo marca con patrones maliciosos — usar `tauri-v2` + docs oficiales |
| `bobmatnyc/claude-mpm-skills@rust-desktop-applications` | skills.sh lo marca con patrones maliciosos — usar `rust-best-practices` |
| `vercel-react-native-skills` | No hay app móvil nativa; removido del repo |
| Azure skills (microsoft) | Circulo no usa Azure |
| Lark/Feishu skills | Integración enterprise CN; fuera de scope |
| Marketing skills (SEO, copywriting) | Producto devtool, no landing growth aún |
| Skills con &lt;100 installs sin repo conocido | Riesgo de calidad baja (ver `find-skills` guidelines) |

---

*Última actualización: julio 2026 — installs aproximados desde skills.sh leaderboard.*