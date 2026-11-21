# Circulo — UX Specification (v1.0)

- **Status:** Draft v1.0 (2026-11-20)
- **Related:** [PRD](prd.md) · [UI](ui.md) · [Flow](flow.md)
- **Design source:** Replicate Waku’s UX and interaction patterns as closely as possible. Later personal adjustments are expected. Ideas and UX patterns only — no code copy (GPL-3.0).

## 1. Principles

1. **The stream is the product.** Watching the agent work live is the core loop. Latency and scroll behavior are sacred.
2. **Calm by default.** Reasoning and tool noise can collapse. The answer is the star. Nothing blinks unless the agent is working.
3. **One surface.** Projects, sessions, transcript, controls, and permissions live in one window (Waku-style).
4. **Honest state.** Starting, running, retrying, error, and permission-waiting are always visible.
5. **Shared control.** Model, reasoning effort, and access modes feel the same no matter which agent is running.
6. **Keyboard first.** Match Waku’s keyboard-first philosophy where practical.

## 2. Layout (target — Waku-inspired)

```
┌─ Custom titlebar / app bar ────────────────────────────┐
│ Circulo          project · session          [status]   │
├──────────────┬───────────────────────────┬─────────────┤
│ Sidebar      │                           │ Right panel │
│ Projects     │        TRANSCRIPT         │ (optional)  │
│ Sessions     │  user / thinking / tools  │ Files       │
│ (grouped,    │  answer (markdown)        │ Diff        │
│  folders,    │  usage footer             │ Terminal    │
│  pin/archive)│                           │ Browser     │
│              │  ┌─ Permission card ────┐  │             │
│              │  │                      │  │             │
│              │  ├─ Composer ───────────┤  │             │
│              │  │ message… [model] [↑] │  │             │
│              │  └──────────────────────┘  │             │
└──────────────┴───────────────────────────┴─────────────┘
```

- **Left sidebar**: projects + sessions (date groups, folders, pinning, archiving, drag-reorder — as in Waku).
- **Center**: virtualized transcript with pin-to-bottom + jump-to-latest, plus composer.
- **Right panel** (Waku-style): tabs for Files, Diff, Terminal, Browser, File Editor. Can be collapsed.
- Navigation rail / minimap for long transcripts (nice-to-have in v1.0).

## 3. Key interactions (match Waku where possible)

- Enter to send, Shift+Enter for newline
- Cmd/Ctrl+Enter to steer mid-turn; queue follow-ups while working
- Model and reasoning effort pickers always available (shared surface)
- Permission / question cards appear above the composer
- Abort with button or Escape
- Git rewind / checkpoints from session or turn context menu
- Keyboard navigation throughout (sessions, composer, panels)

## 4. States

| State | Surface |
|---|---|
| Empty | Centered “Add project” |
| Starting | Banner + spinner |
| Streaming | Live parts + working indicator / pixel-style loader if desired |
| Permission pending | Card above composer |
| Error | Inline block or banner with retry |
| Reconnecting | Thin top bar |

## 5. Accessibility & motion

- Every mouse control is keyboard reachable
- Honor system reduced-motion settings
- High contrast; support dark / light / system (graphite palette)
- Transcript is a log; live regions only for status
- macOS vibrancy on sidebar where available