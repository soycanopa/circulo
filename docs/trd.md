# Circulo — Technical Requirements Document (v1.0)

- **Status:** Draft v1.0 (2026-11-20)
- **Related:** [PRD](prd.md) · [Flow](flow.md) · [Implementation Plan](implement.md)
- **Stack decision:** Fully built with [MyGo](https://mygo.run/) (Go). The window is native UI (`ui`, including `ui/markdown` for streaming replies). The core is a separate process. A WebView is reserved for a later Browser tab, under the native view.
- **Architecture principle:** Everything is separated like puzzle pieces. UI, core, providers, protocol, persistence and Git are independent modules with clear boundaries. This makes the system easy to maintain, test and scale (including future mobile and remote).

## 1. Stack

| Layer | Choice | Notes |
|---|---|---|
| Shell & UI | MyGo native UI | GPU UI (`ui` package), including `ui/markdown`. Pure Go, no cgo on desktop. No web frontend for the app. |
| Language | Go | ≥ 1.22+ |
| Persistence | SQLite + JSON files | Local-first (same spirit as Waku) |
| Protocol | Neutral internal events + provider adapters | Versioned contract (WebSocket, HTTP or channels) |
| Providers | Adapter per CLI | Native structured protocols (not terminal scraping) |
| Build | `mygo` CLI | Desktop bundles, later mobile |

## 2. Architecture — puzzle pieces

```
┌─────────────────────────────────────┐
│  UI process (MyGo app)              │
│  - presentation state only          │
│  - sidebar / transcript / composer  │
│  - right panel views                │
│  - RPC / channel client             │
└─────────────────┬───────────────────┘
                  │ versioned protocol
                  ▼
┌─────────────────────────────────────┐
│  Core process                         │
│  - project & session registry       │
│  - provider drivers                 │
│  - Git / worktree / checkpoints     │
│  - SQLite + blob store              │
│  - filesystem operations            │
└─────────────────────────────────────┘
         │
         ├── Provider adapters (independent packages)
         │     OpenCode, Claude Code, Codex, …
         ├── Protocol package (shared wire types)
         └── Persistence package
```

**Separation rules (enforced):**
- The UI never imports provider-specific code or talks directly to CLIs.
- Providers never import UI or MyGo.
- The protocol package is the only shared contract.
- Core owns all state, Git and processes. UI owns only presentation and a disposable cache.
- New providers, new UI views or future mobile clients can be added without touching the other pieces.

## 3. Provider driver system

Each provider is its own package implementing a common `Driver` interface:
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
- Owned exclusively by the core process

## 5. Key technical goals for v1.0

- Strong modular boundaries (puzzle style)
- Streaming latency and efficient transcript
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

- MyGo native UI performance under high-frequency streaming (measure on the transcript prototype: long history, pin-to-bottom, selection). A virtualized list only contains the paragraphs it has built, so selecting a whole transcript needs a regular scroll.
- Provider protocol drift (tolerant parsers + live smokes)
- GPL boundary: never copy Waku code; reimplement concepts only
