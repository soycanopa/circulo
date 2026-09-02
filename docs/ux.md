# circuloGo — UX Specification

- **Status:** Draft v0.1 (2026-09-02)
- **Related:** [PRD](prd.md) · [UI](ui.md) (visual spec) · [Flow](flow.md)

## 1. Principles

1. **The stream is the product.** Watching the agent think and work is the core loop —
   latency and scroll behavior are sacred.
2. **Calm by default.** Reasoning and tool noise collapse; the answer is the star.
   Nothing blinks unless the agent is working.
3. **One surface.** Projects, sessions, transcript, permissions live in one window;
   no modal maze. The only modal is folder pick.
4. **Honest state.** Connecting / retrying / error are always visible, never guessed.

## 2. Layout

```
┌────────────────────────────────────────────────────────────────────┐
│ [≡] circuloGo                     project: ~/code/api  [status ●]   │  titlebar h-11
├──────────────┬─────────────────────────────────────────────────────┤
│ [+ New chat] │                                                     │
│──────────────│              TRANSCRIPT (scroll)                    │
│ PROJECTS  ▾  │  ┌ user ─────────────────────────────┐              │
│ ● api        │  │ Fix the failing tests in auth     │              │
│ ○ web        │  └───────────────────────────────────┘              │
│──────────────│  ▸ Thinking                    12s   ← reasoning    │
│ TODAY        │  ⚙ bash · npm test            ✓ done ← tool card    │
│  · Fix auth… │  ⚙ edit · auth/session.ts     ✓ +3 −1               │
│  · Refactor… │  ── Worked for 42s · 17.5k tok · $0.03 ──           │
│ YESTERDAY    │  OK, all 14 tests pass now.       ← answer (md)     │
│  · Migrate…  │                                                     │
│──────────────│  ┌─────────────────────────────────────────────┐    │
│ [⚙ Settings] │  │ Permission: run `rm -rf dist`  [Allow][Deny]│    │
│              │  ├─────────────────────────────────────────────┤    │
│              │  │ ▎ Message circuloGo…                  [↑]   │    │
│              │  └─────────────────────────────────────────────┘    │
└──────────────┴─────────────────────────────────────────────────────┘
   sidebar          transcript + composer (fixed at bottom)
   w-72             flexible
```

- **Sidebar (w-72, collapsible to w-12 icon rail):** New chat button; project switcher
  (dot = adapter status: green running, yellow starting/retrying, red error, gray
  attach/stopped); sessions grouped Today / Yesterday / This week / Older, newest first;
  settings at the bottom.
- **Transcript:** single scroll column, max-w-3xl centered, comfortable line height.
- **Composer:** pinned bottom; grows to ~6 lines then scrolls; stop button replaces send
  while busy; model + agent pickers inline (compact dropdowns); permission cards stack
  directly above it — the eye never leaves the input area (same pattern as waku).

## 3. States

| State | Where | What the user sees |
|---|---|---|
| Empty (no projects) | full screen | Centered: product name, one sentence, `[Add project]` |
| Project starting | sidebar dot + transcript banner | "Starting OpenCode…" with spinner; composer disabled |
| Adapter error | sidebar dot + banner | "OpenCode failed: <detail>" + `[Retry]` `[Show logs]` |
| Empty session | transcript | Centered composer hint: agent name + model, "Ask anything" |
| Streaming | transcript | Live reasoning tail, tool cards updating, pulsing "Working… 12s" |
| Retrying | status strip in transcript | "Provider retrying (attempt 2/…): <reason> · next in 3s" |
| Session error | transcript inline | Red-bordered block: error name + message (FR-21) |
| Permission pending | card above composer | Card + composer stays enabled (can type next prompt) |
| Reconnecting (app↔backend) | thin top bar | "Reconnecting… showing last known state" |

## 4. Parts rendering rules

Order inside an assistant turn is strictly the server's part order.

- **reasoning** — collapsed by default *after* completion; **open while streaming**
  showing the last ~10 lines with fade, streaming monospace-ish text; header
  "Thinking · Ns". Click toggles. Never renders raw inside the answer flow.
- **text (assistant)** — GFM markdown. Code blocks: header (language + copy button,
  turns to ✓ for 2 s), wrapped lines, no horizontal scroll. Lists/tables/links styled
  by the typography preset.
- **tool** — card, one line when collapsed: icon by tool kind (bash/edit/read/search),
  tool name, `state.title` or derived summary, state chip (`running` spinner /
  `✓ completed` / `✕ error` / `… pending`). Expand → input (pretty JSON or command) and
  output (mono, max-h-96 scroll, copy). Error state shows output in red tone.
  Consecutive tool cards group under one collapsible "Worked · N steps" cluster when
  the turn is finished (waku-style fold); streaming keeps them expanded.
- **step-start / step-finish** — not rendered directly; `step-finish` aggregates into
  the turn footer: `Worked for 42s · 17.5k tok · $0.03` (sum tokens/cost of the message).
- **patch** — "Changed files" card: file list with +adds/−dels counts; expand → per-file
  unified diff (mono, syntax-quiet, max-h-96 scroll).
- **agent / subtask** — slim pill: "▸ subagent: explore — <description>".
- **file (user attachments)** — chip with filename above the user bubble.

User messages: plain bubble, right-aligned accent border, editable later (v1).

## 5. Streaming & scroll behavior

- **Pin-to-bottom:** if the user is at the bottom (< 24 px), new content keeps the view
  pinned; any manual scroll up releases the pin and shows a `↓ Jump to latest` pill
  (with count of new lines while away, optional).
- **Stream smoothing:** transcript re-renders capped ~15 Hz (TRD §6); text parts may
  reveal with a 100 ms trailing fade. Respect `prefers-reduced-motion` (no reveal
  animation, instant text).
- **Throttle status timer:** "Working… Ns" ticks at 1 Hz, not per frame.
- **Never scroll-jack:** opening history does not force-scroll if the user is reading.

## 6. Interactions & shortcuts

| Action | Binding |
|---|---|
| Send prompt | `Enter` |
| Newline | `Shift+Enter` |
| New chat | `Cmd+N` |
| Toggle sidebar | `Cmd+B` |
| Focus composer | `Cmd+K` (from anywhere) |
| Jump to latest | `Cmd+↓` or pill click |
| Abort turn | `Esc` while busy (with 500 ms hold-to-confirm if tools running) or Stop button |
| Session context menu | right-click: Rename / Delete (delete asks confirm) |
| Project switcher | click project row → its sessions |

Copy: every code block and tool output has a copy affordance; transcript rows copy as
markdown on selection copy (v1).

## 7. Permission card UX

- Appears above composer with alert icon, title from server (`metadata.command` for
  bash, target file for edit), collapsible detail (≤ 3 lines, mono).
- Buttons: `[Allow once]` (primary), `[Always allow]` (secondary), `[Deny]` (ghost/red).
- While pending: subtle amber border; resolves by server event (`permission.resolved`),
  card animates out. Multiple pending permissions stack, oldest first.
- Deny never blocks the composer; the agent receives the rejection and continues.

## 8. Accessibility & quality bar

- Full keyboard reach: sidebar (↑↓ + Enter), composer, permission buttons.
- ARIA: transcript = `log` region; tool cards = `button` + `aria-expanded`; live regions
  only for status line (not every token — screen-reader noise).
- Contrast: WCAG AA on both themes; dark theme is the default, light fully supported.
