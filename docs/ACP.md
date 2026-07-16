# ACP en Circulo

Circulo implementa el rol de **Client** en el [Agent Client Protocol](https://agentclientprotocol.com/).

## ¿Por qué ACP y no HTTP/SSE?

| Enfoque | Usado por | Pros | Contras |
|---------|-----------|------|---------|
| HTTP + SSE | Palot + `@opencode-ai/sdk` | SDK maduro, muchas APIs | Acoplado a OpenCode |
| ACP + stdio | Zed, JetBrains, Circulo | Multi-agente estándar | Cliente más complejo |

Circulo elige ACP para soportar cualquier CLI compatible: OpenCode, Cline, Grok CLI, Gemini CLI, etc.

## OpenCode

```bash
opencode acp
```

Documentación: https://opencode.ai/docs/acp/

Circulo ejecuta este comando como subproceso al abrir un proyecto. El `cwd` del proceso es la carpeta del proyecto.

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

OpenCode expone modelos en `configOptions` al crear la sesión. Circulo renderiza un `<select>` y llama:

```
session/set_config_option { configId, value }
```

## Tool calls relevantes

| `kind` ACP | UI Circulo |
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

## Límites de ACP y fuentes complementarias

Circulo usa **ACP como canal principal**, pero no asume que el protocolo exponga todo lo que OpenCode (u otros agentes) ofrecen por SDK, CLI o HTTP.

**Principio:** implementar primero con ACP; donde el gap sea estable, complementar con la vía más simple (sin segundo stack de chat).

| Necesidad | ¿ACP? | Alternativa en OpenCode |
|-----------|-------|-------------------------|
| Streaming, tool calls, permisos | ✅ | — |
| Cambio de modelo (`set_config_option`) | ✅ | — |
| Uso de contexto (tokens, %, coste) | ✅ `usage_update` | — |
| Desglose por categoría (MCP, Skills, …) | ❌ | `GET /session/:id/context` (servidor HTTP) |
| Undo / compact conversación | ❌ o limitado | SDK `@opencode-ai/sdk` |
| Export transcript | parcial | `opencode export` (CLI) |
| Admin MCP, attachments, settings | ❌ | SDK + `opencode serve` |

**Implementación prevista (anotación):** la capa Rust de Circulo puede, por feature:

1. Seguir solo ACP (preferido).
2. Invocar **CLI** del agente ya instalado (`opencode …`).
3. Llamar **HTTP local** al servidor OpenCode si está levantado.
4. Usar **SDK** solo si CLI/HTTP no bastan (más acoplamiento).

No todo se podrá implementar **solo** con ACP; eso es esperado y no implica abandonar el protocolo.

**Ejemplo ya en el código:** `src/lib/context-window.ts` parsea `usage_update` para totales; el desglose por categoría queda preparado en tipos/parser pero requiere otra fuente cuando se implemente el fallback.

## Referencias

- Spec: https://agentclientprotocol.com/protocol/v1/overview
- Rust SDK: https://docs.rs/agent-client-protocol
- Registry de agentes: https://agentclientprotocol.com/get-started/agents
- OpenCode ACP: https://opencode.ai/docs/acp/
- OpenCode SDK: https://opencode.ai/docs/sdk/