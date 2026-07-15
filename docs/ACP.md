# ACP en Forge

Forge implementa el rol de **Client** en el [Agent Client Protocol](https://agentclientprotocol.com/).

## ¿Por qué ACP y no HTTP/SSE?

| Enfoque | Usado por | Pros | Contras |
|---------|-----------|------|---------|
| HTTP + SSE | Palot + `@opencode-ai/sdk` | SDK maduro, muchas APIs | Acoplado a OpenCode |
| ACP + stdio | Zed, JetBrains, Forge | Multi-agente estándar | Cliente más complejo |

Forge elige ACP para soportar cualquier CLI compatible: OpenCode, Cline, Grok CLI, Gemini CLI, etc.

## OpenCode

```bash
opencode acp
```

Documentación: https://opencode.ai/docs/acp/

Forge ejecuta este comando como subproceso al abrir un proyecto. El `cwd` del proceso es la carpeta del proyecto.

## Flujo de un turno

1. UI → `send_prompt(text, contextPaths)`
2. Rust adjunta bloques de contexto para cada `@file`
3. Rust → `session/prompt` (ACP)
4. Agente emite `session/update` (chunks, tool_call, tool_call_update)
5. Rust re-emite como `acp:session_update`
6. Si necesita permiso → `session/request_permission` → UI bloqueada
7. Al terminar → `acp:prompt_complete`

## Permisos

ACP envía opciones como:

- `allow_once` → **Aprobar**
- `reject_once` → **Denegar**

El usuario debe responder antes de que el agente continúe. Esto es intencional.

## Modelos

OpenCode expone modelos en `configOptions` al crear la sesión. Forge renderiza un `<select>` y llama:

```
session/set_config_option { configId, value }
```

## Tool calls relevantes

| `kind` ACP | UI Forge |
|------------|----------|
| `read` | Preview de archivo |
| `search` | Resultados grep/glob |
| `execute` | Terminal con ANSI |
| `edit` | InlineDiffBlock |
| `fetch` | URL + preview |

## Agentes futuros

| Agente | Comando ACP | Estado |
|--------|-------------|--------|
| OpenCode | `opencode acp` | ✅ MVP |
| Cline | `cline acp` | 🔜 |
| Grok CLI | según docs | 🔜 |
| Agy/Antigravity | N/A | ❌ sin ACP nativo aún |

## Referencias

- Spec: https://agentclientprotocol.com/protocol/v1/overview
- Rust SDK: https://docs.rs/agent-client-protocol
- Registry de agentes: https://agentclientprotocol.com/get-started/agents