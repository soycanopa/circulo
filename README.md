# Circulo

Desktop AI agent orchestrator built on the **[Agent Client Protocol (ACP)](https://agentclientprotocol.com/)**.

Circulo is a native **Tauri** client for coding agents that speak ACP over stdio. Visual language is inspired by [Palot](https://github.com/ItsWendell/palot); the backend is **not** OpenCode HTTP/SSE — it is pure ACP.

**First agent:** [OpenCode](https://opencode.ai) via `opencode acp`.

## Stack

| Layer | Technology |
|-------|------------|
| Core | Tauri v2 + Rust |
| Protocol | ACP (`agent-client-protocol`) |
| Runtime | Bun |
| Frontend | React 19 + Vite + Tailwind v4 |
| State | Jotai |

## Docs

- [PRD](./docs/PRD.md) — product requirements  
- [TRD](./docs/TRD.md) — technical design  
- [UX](./docs/UX.md) — UI direction  
- [Flows](./docs/FLOWS.md) — core user flows  
- [ACP](./docs/ACP.md) — protocol mapping  
- [QA](./docs/QA.md) — manual test checklist  

## Requirements

- Bun 1.3+
- Rust stable (`rustup`)
- macOS Xcode CLT (for Tauri)
- OpenCode CLI on `PATH` (for agent features)

```bash
bun --version
rustc --version
opencode --version
opencode acp --help
```

## Quick start

```bash
cd ~/Desktop/circulo
bun install
bun run tauri dev
```

## Scripts

| Command | Description |
|---------|-------------|
| `bun run dev` | Vite only (`:1420`) |
| `bun run tauri dev` | Full desktop app |
| `bun run build` | Frontend production build |
| `bun run check-types` | `tsc --noEmit` |
| `bun run tauri build` | Packaged app |

## License

MIT
