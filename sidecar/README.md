# Sidecar Node (puente temporal)

Durante la migración **Electron → Tauri**, el proceso principal original de 1Code
(servidor tRPC, SQLite, terminal `node-pty`, SDKs de Claude/Codex, etc.) se ejecuta
como **sidecar** en Bun/Node.

- **Puerto por defecto:** `21337`
- **Variable:** `CIRCULO_SIDECAR_PORT`
- **Datos de usuario:** `CIRCULO_USER_DATA` (lo establece Tauri)

Tauri arranca este proceso en `setup` (`src-tauri/src/commands/sidecar.rs`).

## Estado

- v0.1.0: servidor HTTP tRPC mínimo con `health` y arranque de base de datos (cuando
  el mock de `electron` es suficiente).
- El resto de routers sigue importando APIs de Electron y se migran a Rust o a
  comandos Tauri según `docs/MIGRACION.md`.

## Ejecución manual

```bash
export CIRCULO_USER_DATA="$HOME/.circulo"
export CIRCULO_WORKSPACE="$(pwd)"
bun run sidecar/entry.ts
```

## Salida prevista

Este directorio desaparecerá por completo cuando todos los módulos estén en Rust
o expuestos como plugins oficiales de Tauri con sidecars empaquetados solo donde
sea inevitable (p. ej. binarios de agentes).