# Guía de desarrollo

## Setup inicial

```bash
cd ~/Desktop/circulo
bun install
source ~/.cargo/env   # si Rust no está en PATH
```

### OpenCode

```bash
# Instalar (ver https://opencode.ai)
curl -fsSL https://opencode.ai/install | bash

# Configurar proveedor (una vez)
opencode
```

## Ejecutar en desarrollo

```bash
bun run tauri dev
```

Esto levanta Vite en `:1420` y compila/ejecuta el binario Tauri.

## Estructura de commits sugerida

1. `chore:` scaffold / tooling
2. `feat(acp):` cambios en protocolo Rust
3. `feat(ui):` componentes React
4. `feat(security):` permisos
5. `docs:` documentación

## Debugging ACP

### Logs Rust

```bash
RUST_LOG=info bun run tauri dev
```

### Ver eventos en el frontend

Los eventos Tauri se escuchan en `src/hooks/use-acp-session.ts`. Puedes añadir temporalmente:

```ts
console.log("acp:session_update", payload)
```

### Probar OpenCode ACP manualmente

```bash
cd /ruta/a/tu/proyecto
opencode acp
```

## Añadir un nuevo agente

1. Registrar comando en `src-tauri/src/agents/mod.rs`
2. Exponer selector en UI (futuro)
3. Verificar que el agente implemente:
   - `initialize`
   - `session/new`
   - `session/prompt`
   - `session/update` notifications
   - `session/request_permission`

## Typecheck

```bash
bun run check-types
```

## Build release

```bash
bun run tauri build
```

El artefacto queda en `src-tauri/target/release/bundle/`.

## Problemas comunes

| Problema | Solución |
|----------|----------|
| `opencode: command not found` | Instalar CLI y añadir `~/.opencode/bin` al PATH |
| Ventana en blanco | Verificar que Vite corre en puerto 1420 |
| Permisos colgados | Confirmar que `PermissionCard` llama `respond_permission` |
| Sin modelos en selector | Verificar proveedores en `~/.config/opencode/` |

## Paper MCP (diseño visual)

Paper Desktop expone un servidor MCP local al abrir un archivo.

1. Instalar [Paper Desktop](https://paper.design/downloads)
2. Abrir el mock de Circulo en Paper
3. Conectar el MCP en tu herramienta de agente

### Grok CLI

```bash
grok mcp add --transport http paper http://127.0.0.1:29979/mcp
grok mcp doctor paper
```

### Antigravity

En `~/.gemini/antigravity/mcp_config.json`:

```json
{
  "mcpServers": {
    "paper": {
      "serverUrl": "http://127.0.0.1:29979/mcp"
    }
  }
}
```

Docs: https://paper.design/docs/mcp

**Nota:** Paper/Figma/Craft están deshabilitados dentro de Circulo. El agente OpenCode que lanza Circulo usa `OPENCODE_CONFIG_CONTENT` para bloquear MCPs de diseño; úsalos solo en Grok/Cursor/Antigravity, no en el chat de Circulo.

## Multi-sesión ACP

Commands Tauri expuestos:

- `list_sessions` — `session/list`
- `create_session` — `session/new`
- `load_session` — `session/load`
- `close_session` — `session/close`

Eventos:

- `acp:sessions_updated` — lista de sesiones + sesión activa
- `acp:session_ready` — cambio de sesión activa (limpia chat en frontend)

Si OpenCode no anuncia `session/list`, el sidebar degrada a una sola sesión.
