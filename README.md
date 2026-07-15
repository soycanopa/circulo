# Forge

Orquestador de escritorio para agentes de IA basado en **ACP (Agent Client Protocol)**.

Forge es una alternativa ligera a interfaces como Palot/Codex, construida con **Tauri v2 + Rust** en el core y **React + Bun** en el frontend. El primer agente soportado es **OpenCode** vía `opencode acp`.

## Características (MVP)

- Gestión del ciclo de vida del agente al abrir/cerrar un proyecto
- Chat con streaming en tiempo real (eventos ACP → UI)
- Gate de permisos **Aprobar / Denegar**
- Visualización de tool calls (read, search, execute, edit, fetch)
- Menciones `@` para archivos del workspace
- Diffs inline básicos (rojo/verde)
- Selector de modelos vía `configOptions` de ACP

## Requisitos

- [Bun](https://bun.sh) 1.3+
- [Rust](https://rustup.rs) stable
- Dependencias de Tauri para macOS (Xcode CLI Tools)
- [OpenCode CLI](https://opencode.ai) en `PATH`
- Al menos un proveedor de modelos configurado en OpenCode

```bash
# Verificar OpenCode
opencode --version
opencode acp --help
```

## Inicio rápido

```bash
cd ~/Desktop/forge
bun install
bun run tauri dev
```

1. Pulsa **Abrir proyecto** y selecciona una carpeta.
2. Forge lanzará `opencode acp` automáticamente.
3. Escribe en el chat; usa `@` para referenciar archivos.
4. Aprueba o deniega permisos cuando el agente lo solicite.

## Scripts

| Comando | Descripción |
|---------|-------------|
| `bun run dev` | Solo frontend Vite |
| `bun run tauri dev` | App de escritorio completa |
| `bun run build` | Build frontend |
| `bun run check-types` | Typecheck TypeScript |

## Documentación

- [Arquitectura](docs/ARCHITECTURE.md)
- [ACP en Forge](docs/ACP.md)
- [Guía de desarrollo](docs/DEVELOPMENT.md)
- [AGENTS.md](AGENTS.md) — instrucciones para agentes de código

## Roadmap

- [ ] Multi-agente (Cline, Grok CLI, Gemini CLI)
- [ ] Multi-proyecto / multi-sesión
- [ ] Panel lateral de review (estilo Palot)
- [ ] Adaptador para Agy/Antigravity cuando exponga ACP

## Licencia

MIT — UI inspirada en [Palot](https://github.com/ItsWendell/palot) (MIT).