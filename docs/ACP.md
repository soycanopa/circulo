# Circulo — ACP mapping

Primary references:

- [Introduction](https://agentclientprotocol.com/get-started/introduction)  
- [Overview](https://agentclientprotocol.com/protocol/v1/overview)  
- [OpenCode ACP](https://opencode.ai/docs/acp/)

## Transport

Local agent subprocess, **JSON-RPC 2.0 over stdio**.

## Methods we implement (MVP)

| Direction | Method | Purpose |
|-----------|--------|---------|
| C→A | `initialize` | Version + capability negotiation |
| C→A | `session/new` | Create session with absolute `cwd` |
| C→A | `session/prompt` | User turn |
| C→A | `session/set_config_option` | Model/mode when offered |
| A→C | `session/update` | Stream chunks, tools, plans, usage |
| A→C | `session/request_permission` | Tool permission gate |

## Optional (post-MVP)

`session/load`, `session/list`, `session/close`, `session/cancel`, client `fs/*`, `terminal/*`, elicitation.

## Conventions

- File paths are **absolute**.  
- Property keys: `camelCase`.  
- Discriminator strings: `snake_case`.  
- User-readable text: Markdown.

## OpenCode entrypoint

```bash
opencode acp --cwd /absolute/path/to/project
```

Circulo resolves the binary (PATH + common install locations) and never uses HTTP/SSE OpenCode server for the primary chat path.

## Multi-agent future

Agent binaries are selected via a registry (`id`, command, args, env). Chat UI and parser stay ACP-shaped so new agents plug in without rewriting the shell.
