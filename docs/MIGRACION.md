# Migración Electron/Node → Rust + Tauri v2

Documento vivo para la reconstrucción de **Círculo** sobre la base UI/servicios de
[1Code](https://github.com/21st-dev/1code) (importado en `9f1bc76fa4372c18c565b5a4f8daf38ae3595f0e`).

**Regla:** no eliminar ni simplificar funcionalidad en esta fase; migrar módulo a módulo.

## Leyenda de estado

| Estado | Significado |
|--------|-------------|
| ✅ | Migrado a Rust/Tauri y usable en el camino principal |
| 🟡 | Parcial: bridge Tauri + sidecar Node temporal |
| 🟠 | Sidecar Node (sin cambio funcional respecto a 1Code) |
| ⏳ | Pendiente |

## Resumen v0.1.0 (esta PR)

| Área | Estado | Notas |
|------|--------|-------|
| Shell de ventana (Tauri) | ✅ | `src-tauri/`, ventana 1400×900 |
| UI React heredada | 🟡 | `src/renderer/` vía Vite ; tRPC HTTP al sidecar |
| `desktopApi` | 🟡 | `src/renderer/lib/circulo/desktop-api-tauri.ts` — ventana, clipboard, openExternal |
| Sidecar tRPC | 🟠 | `sidecar/` — mock `electron`, router HTTP cuando carga |
| Proceso principal Electron | 🟠 | `src/main/index.ts` — referencia + sidecar |
| Preload / `trpc-electron` | 🟠 | Sigue para `dev:electron:legacy` |
| Git / worktrees | 🟠 | `src/main/lib/git/**` en sidecar |
| Terminal PTY | 🟠 | `node-pty` en sidecar |
| Agentes Claude/Codex | 🟠 | SDKs en sidecar |
| SQLite / Drizzle | 🟡 | DB en sidecar; rutas via `CIRCULO_*` |
| Auto-actualización | ⏳ | Sustituir `electron-updater` por Tauri updater |
| Auth OAuth | ⏳ | `auth-manager.ts` |
| Sentry | ⏳ | Tauri / alternativa |
| Multi-ventana | ⏳ | `window-manager.ts` |

## Orden granular propuesto (versiones)

### v0.1.x — Base Tauri (esta entrega)

| ID | Módulo / ruta | Estado |
|----|----------------|--------|
| 0.1.0 | Reestructura repo (`legacy/circulo`, UI en raíz) | ✅ |
| 0.1.0 | Licencia NOTICE + LICENSE-1CODE | ✅ |
| 0.1.1 | Sidecar HTTP tRPC estable | 🟡 (health OK; router completo depende de nativos) |
| 0.1.2 | `desktopApi` ventana + zoom + fullscreen | 🟡 |
| 0.1.3 | CI `tauri build` + `vite build` | ⏳ |

### v0.2.x — IPC y almacenamiento

| ID | Módulo | Destino | Estado |
|----|--------|---------|--------|
| 0.2.0 | `src/main/lib/db/**` | Rust (`sqlx` + migraciones) o plugin SQLite | ⏳ |
| 0.2.1 | `src/main/lib/config.ts` | Tauri `app_data` + Rust settings | ⏳ |
| 0.2.2 | `auth-store.ts` / `credential-manager.ts` | Rust + keyring | ⏳ |
| 0.2.3 | `trpc` transport | HTTP local Rust o `tauri::ipc` tipado | 🟡 |

### v0.3.x — Git y archivos

| ID | Módulo | Destino | Estado |
|----|--------|---------|--------|
| 0.3.0 | `lib/git/worktree.ts` | Crate Rust (git2 o `git` CLI) | ⏳ |
| 0.3.1 | `lib/git/watcher/**` | notify + eventos Tauri | ⏳ |
| 0.3.2 | `lib/fs/**` | `tauri-plugin-fs` + validación Rust | ⏳ |
| 0.3.3 | `trpc/routers/files.ts` | Comandos Rust | ⏳ |

### v0.4.x — Terminal

| ID | Módulo | Destino | Estado |
|----|--------|---------|--------|
| 0.4.0 | `lib/terminal/**` | `portable-pty` / `tauri-plugin-shell` PTY | ⏳ |
| 0.4.1 | Renderer xterm | Igual UI, WebSocket a Rust | ⏳ |

### v0.5.x — Agentes (Claude / Codex)

| ID | Módulo | Destino | Estado |
|----|--------|---------|--------|
| 0.5.0 | `trpc/routers/claude.ts` | Spawn/stream en Rust | ⏳ |
| 0.5.1 | `trpc/routers/codex.ts` | Sidecar mínimo si el SDK exige Node | 🟠 |
| 0.5.2 | `lib/claude/**` | Adaptadores Rust + pipes | ⏳ |
| 0.5.3 | Binarios en `resources/bin` | Empaquetado Tauri | ⏳ |

### v0.6.x — Plataforma y pulido

| ID | Módulo | Destino | Estado |
|----|--------|---------|--------|
| 0.6.0 | `lib/auto-updater.ts` | `tauri-plugin-updater` | ⏳ |
| 0.6.1 | `lib/analytics.ts` | Opt-in + Rust batch | ⏳ |
| 0.6.2 | `lib/platform/**` | `cfg(target_os)` en Rust | ⏳ |
| 0.6.3 | Deep links / protocol | Tauri deep-link plugin | ⏳ |
| 0.6.4 | Retirar sidecar Node | — | ⏳ |

## Inventario por carpeta `src/main`

| Carpeta / archivo | Rol | Migración |
|-------------------|-----|-----------|
| `index.ts` | Ciclo de vida app | Rust `src-tauri/src/lib.rs` + plugins |
| `auth-manager.ts` | OAuth | ⏳ Rust |
| `auth-store.ts` | Tokens cifrados | ⏳ Rust keyring |
| `windows/main.ts` | Ventanas + tRPC IPC | ✅ Tauri + HTTP sidecar |
| `windows/window-manager.ts` | Multi-ventana | ⏳ |
| `lib/db/**` | SQLite Drizzle | 🟡 sidecar |
| `lib/trpc/routers/**` | API backend | 🟠 sidecar → Rust por router |
| `lib/git/**` | Git, worktrees, PR | 🟠 sidecar → v0.3 |
| `lib/terminal/**` | PTY | 🟠 sidecar → v0.4 |
| `lib/claude/**` | Claude SDK | 🟠 sidecar → v0.5 |
| `lib/auto-updater.ts` | Updates | ⏳ v0.6 |
| `lib/cli.ts` | CLI sistema | ⏳ |
| `lib/mcp-auth.ts` | MCP OAuth | ⏳ |
| `lib/oauth.ts` | OAuth genérico | ⏳ |
| `lib/platform/**` | macOS/Win/Linux | ⏳ v0.6 |
| `lib/plugins/**` | Plugins | ⏳ |
| `lib/ollama/**` | Ollama | 🟠 sidecar |

## Sidecar (puente temporal)

- **Código:** `sidecar/entry.ts`, `sidecar/electron-mock.ts`
- **Arranque:** automático desde Tauri (`sidecar_start`)
- **Puerto:** `21337`
- **Retirada:** cuando todos los ítems ≥ v0.6.4 estén ✅

## Comandos de desarrollo

```bash
# UI sola (navegador — sin desktopApi completo)
bun run dev:ui

# Sidecar manual
CIRCULO_USER_DATA=~/.circulo bun run sidecar

# App Tauri (recomendado)
CIRCULO_TAURI=1 bun install
bun run tauri:dev

# Electron heredado (referencia)
bun run dev:electron:legacy
```

## Verificación en CI / agente

- [x] `cargo check` en `src-tauri` (Linux, Rust 1.90)
- [x] `vite build` renderer (requiere `NODE_OPTIONS=--max-old-space-size=8192`)
- [ ] `tauri dev` / ventana gráfica (no verificado en este agente; depende de display)
- [ ] Sidecar con `better-sqlite3` — requiere `node-gyp rebuild` nativo en cada máquina
- [ ] `node-pty` / binarios de agentes (Claude/Codex) — bits específicos de SO

## Legado Círculo (GPUI/Rust)

El código anterior vive en `legacy/circulo/` (MIT). No se borra; sirve de referencia
para protocolo, proveedores y documentación de producto.