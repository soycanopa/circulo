# Circulo — Technical Requirements Document (v1.0)

- **Status:** Draft v1.0 (2026-11-21)
- **Related:** [PRD](prd.md) · [Flow](flow.md) · [Implementation Plan](implement.md)
- **Stack decision:** Rust + GPUI (same technology as Waku). Native GPU-accelerated UI.
- **Architecture principle:** Everything is separated like puzzle pieces. UI, core, providers, protocol, persistence and Git are independent crates/modules with clear boundaries. This makes the system easy to maintain, test and scale (including future mobile and remote).

## 1. Stack

| Layer | Choice | Notes |
|---|---|---|
| Shell & UI | GPUI | GPU-accelerated native UI (same as Waku / Zed). |
| Language | Rust | Stable, edition 2021 or later |
| Persistence | SQLite + JSON files | Local-first (same spirit as Waku) |
| Protocol | Neutral events + provider adapters | Versioned, transport-agnostic contract |
| Transport | WebSocket, JSON text frames | `ws://` on loopback; `wss://` behind TLS for remote |
| Providers | Adapter per CLI | Native structured protocols (not terminal scraping) |
| Build | cargo | Desktop bundles (macOS .app, Linux, Windows) |

## 2. Architecture — puzzle pieces

The core runs as its own process (`circulo-daemon`) from day one. The desktop, and later mobile and remote clients, connect to it over the protocol. That is what makes the same core usable everywhere without moving boundaries: a client that can speak the protocol does not care where the daemon runs.

Transport: a WebSocket, one JSON message per text frame, `ws://` on loopback and `wss://` once TLS fronts the daemon. The protocol crate owns the message types; the WebSocket layer only carries them, so remote is a deployment change (TLS terminator, private network), not a contract change. The handshake carries the protocol version and a token the daemon prints when it starts; remote adds TLS and header authentication in front of it.

```
┌─────────────────────────────────────┐
│  UI (GPUI app)                      │
│  - presentation state only          │
│  - sidebar / transcript / composer  │
│  - right panel views                │
│  - RPC / channel client             │
└─────────────────┬───────────────────┘
                  │ versioned protocol
                  ▼
┌─────────────────────────────────────┐
│  Core / Daemon (own process)        │
│  - project & session registry       │
│  - provider drivers                 │
│  - Git / worktree / checkpoints     │
│  - SQLite + blob store              │
│  - filesystem operations            │
└─────────────────────────────────────┘
         │
         ├── Provider adapters (independent crates)
         │     OpenCode, Claude Code, Codex, …
         ├── Protocol crate (shared wire types)
         └── Persistence crate
```

**Separation rules (enforced):**
- The UI never imports provider-specific code or talks directly to CLIs.
- Providers never import UI or GPUI.
- The protocol crate is the only shared contract.
- Core owns all state, Git and processes. UI owns only presentation and a disposable cache.
- New providers, new UI views or future mobile clients can be added without touching the other pieces.

## 3. Provider driver system

Each provider is its own crate implementing a common `Driver` trait:
- Start / attach session
- Send prompt / steer / queue
- Stream normalized events
- Abort
- Model / reasoning effort discovery

Normalized events travel only through the protocol.

## 4. Persistence

- SQLite for sessions, messages, parts, projects
- JSON for app settings
- Content-addressed blobs for attachments
- Owned exclusively by the core

## 5. Key technical goals for v1.0

- Strong modular boundaries (puzzle style, crates)
- Streaming latency and efficient GPUI virtualized transcript
- Conversation-aware Git checkpoints
- Model switching and reasoning effort across providers
- Message queue + steering
- Clean adapter boundary so new providers require zero UI changes

## 6. Testing

- Each piece tested in isolation
- Protocol + adapter fixture tests
- Integration tests with real CLIs
- Manual E2E checklist

## 7. Security & privacy

- Local only by default
- No telemetry
- Future remote access will expose only the protocol surface, never the providers directly

## 8. Risks

- GPUI learning curve and high-frequency streaming performance (early prototype required)
- Provider protocol drift (tolerant parsers + live smokes)
- GPL boundary: never copy Waku code; reimplement concepts only
