# Círculo

Cliente de escritorio para orquestar agentes de código (Claude Code, Codex y otros)
en paralelo, con worktrees, terminal integrada y UI de conversación.

**Stack:** Rust + [Tauri v2](https://v2.tauri.app/) + React/TypeScript (UI heredada de
[1Code](https://github.com/21st-dev/1code), Apache-2.0 — ver `NOTICE` y `LICENSE-1CODE`).

> Migración en curso: ver [`docs/MIGRACION.md`](docs/MIGRACION.md).

## Desarrollo rápido

Requisitos: **Rust 1.83+**, **Bun**, dependencias nativas para `better-sqlite3` y
`node-pty` (sidecar temporal).

```bash
git clone https://github.com/soycanopa/circulo.git
cd circulo
CIRCULO_TAURI=1 bun install
bun run tauri:dev
```

Solo la UI (navegador):

```bash
bun run dev:ui
```

## Estructura

- `src/renderer/` — interfaz React
- `src-tauri/` — shell Tauri y comandos Rust
- `src/main/` — proceso heredado (Electron/sidecar)
- `sidecar/` — puente Node temporal
- `legacy/circulo/` — código y docs de la línea GPUI anterior (referencia)

## Licencia

- Círculo: **MIT** (`LICENSE`)
- Componente basado en 1Code: **Apache-2.0** (`LICENSE-1CODE`)

## Legado

El cliente Rust+GPUI anterior no se elimina: vive en `legacy/circulo/`.