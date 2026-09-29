# Circulo — UI Specification

- **Status:** v0.2 (2026-09-29) — rewritten from the shipped build. Supersedes
  Draft v0.1 (2026-09-02), which described a neutral-zinc palette and a
  `components/{layout,sidebar,chat}/` tree that the 2026-09-18 Circulo Paper pivot
  replaced. **§2–§4 are contract** (durable rules, changes need a reason);
  **§5 is inventory as of 2026-09-29** (verify against the files, not against this
  doc, before relying on it).
- **Related:** [UX](ux.md) (behavior) · [Flow](flow.md) · [AGENTS.md](../AGENTS.md)
- **Source of truth for every claim below:** `frontend/src/index.css` (tokens),
  `frontend/src/components/layout/AppShell.tsx` (geometry), the file named in each
  inventory row (behavior). When the code and this doc disagree, the code is right
  and this doc is a bug — fix it in the same commit as whatever changed it.

## 1. Design language

The visual layer is a **replica of the owner's Circulo Paper design**: tokens are
a 1:1 mirror of the Paper file, and token names are kept identical to the design so
utility classes read like the JSX (`bg-bg-sidebar`, `text-text-tertiary`).

- **Dark-first, one palette.** No `prefers-color-scheme` branch, no `.light`
  overrides — the `.dark` class on `<html>` is inert on purpose. A light theme is a
  v1 candidate, not a v0 commitment ([ux.md](ux.md) §8).
- **One accent.** `--accent-cir` is the only decorative hue. Status colors are
  reserved for meaning: `success` = running, `warning` = permission/retry/pressure,
  `danger` = error. An earlier decision to keep the palette neutral ("violet
  rejected") was reversed by the pivot.
- **Content is selectable, chrome is not.** `body { user-select: none }` with an
  opt-in for `pre`, `code`, and `[data-selectable]`: the transcript is content; the
  interface around it is not.
- **The document never scrolls.** The app is a fixed `h-screen` shell and
  `html, body` lock `overflow: hidden`; without it WKWebView reports a ~17px
  overflow and lets the wheel move the whole interface. Every scroller stops at its
  own edge (`overscroll-behavior: none`).

## 2. Tokens (contract)

Declared once in `frontend/src/index.css` `:root`, mapped into Tailwind v4 via
`@theme inline`, then aliased onto the shadcn contract names so shadcn components
inherit the Circulo palette instead of fighting it.

| Role | Token | Value |
|---|---|---|
| App surface (frame) | `--bg-app` | `#17171a` |
| Sidebar | `--bg-sidebar` | `#1c1c1f` |
| Main surface | `--bg-main` | `#1c1c1f` (owner call: the sidebar tone) |
| Code | `--bg-code` | `#141417` |
| Popover | `--bg-popover` | `#1f1f21` |
| Hover | `--bg-hover` | `#2b2b30` |
| Danger surface | `--bg-danger` | `#472429` |
| Border | `--border` | `#38383d` |
| Border strong / input | `--border-strong` | `#47474d` |
| Text primary | `--text-primary` | `#ebebed` |
| Text secondary | `--text-secondary` | `#9e9ea6` |
| Text tertiary | `--text-tertiary` | `#808087` |
| Accent | `--accent-cir` | `#7378f2` (hover `#8589f5`) |
| Streaming caret/text | `--streaming` | `#a3a8f7` |
| Success / Warning / Danger | | `#73c78c` / `#dca85e` / `#e66b6b` |
| Diff add / del | | `#386b47` / `#73383d` |

Type scale is explicit, not Tailwind's default: `xs 11 · sm 12 · base 13 · md 14 ·
lg 16 · xl 20 · 2xl 24`. Radii: `xs 4 · sm 6 · md 8 · lg 12 · xl 16`; the shadcn
`--radius` alias is `0.5rem`. Mono is `ui-monospace, Menlo, "SF Mono"`.

**Rule:** never introduce a raw hex or a Tailwind palette color in a component. If
a value is missing, add a token; if no token fits, the design may need a decision
rather than a one-off color.

## 3. Frame geometry (contract)

`AppShell` implements the Paper frame "Circulo / General":

```
┌─ 8px margin (p-2) ───────────────────────────────────────┐
│ ┌─ 40px app bar (h-10) ─────────────────────────────────┐ │
│ │ [74px inset: traffic lights + toggle] · title · acts │ │
│ ├─ gap 8px ─────────────────────────────────────────────┤ │
│ │ ┌── sidebar ──┐ ┊ ┌── main (bg-bg-main) ─────────────┐ │ │
│ │ │ draggable   │┊ │ ┌──────────────────────────────┐ │ │ │
│ │ │ 200–480px   │┊ │ │ transcript (or empty state)  │ │ │ │
│ │ │ default 260 │┊ │ │ composer + context gauge    │ │ │ │
│ │ │             │┊ │ └──────────────────────────────┘ │ │ │
│ │ │             │┊ │ ┌─ terminal card (collapsible) ┐ │ │ │
│ │ └─────────────┘ ┊ │ │ tabs + xterm, max-h 240px   │ │ │ │
│                    ┊ │ └────────────────────────────┘ │ │ │
│                    ┊ └───────────────────────────────┘ │ │
│                    └───────────────────────────────────┘ │
└──────────────────────────────────────────────────────────┘
```

- The app-bar left slot is padded `pl-[74px]` to clear the macOS traffic lights
  (`InvisibleTitleBarHeight: 44` in `main.go`); it matches the sidebar width so the
  context title stays aligned when the sidebar slides.
- The sidebar width is drag-resizable (200–480px, default 260) and persists in
  `localStorage` under `circulo.sidebarWidth`; double-click the handle resets it.
  The handle is an invisible `role="separator"` living in the 8px gap.
- Both the sidebar and main surfaces carry the Paper bottom glow (`circulo-glow`
  utility) over a `GlowPixels` texture, in that layer order: surface → pixels →
  glow → content.
- **Wheel routing is a contract, not a detail.** A wheel event outside a
  self-scrolling surface scrolls the transcript: `aside` scrolls itself, the
  terminal (`[data-native-scroll]`), the transcript (`[data-chat-scroll]`) and any
  scrollable ancestor handle themselves, and a portal-mounted Radix menu stops the
  walk so model/branch pickers never fling the transcript. The ancestor walk is in
  `useTranscriptWheel` — if you add a scroller, it must be reachable from the DOM
  or it will steal the wheel.

## 4. Surfaces and file layout (contract)

Organized by **feature**, not by component role:

```
frontend/src/
├─ components/        shared shell + the shadcn set only
│  ├─ layout/         AppShell, GlowPixels
│  ├─ ui/             shadcn: alert-dialog, badge, button, collapsible,
│  │                  dropdown-menu, popover
│  └─ ProviderIcon    the opencode / omp marks
├─ features/
│  ├─ chat/           Transcript, Message, Composer, ChatAppBar, richInput,
│  │  │               hooks/usePinnedScroll
│  │  ├─ markdown/    MarkdownView, CodePanel, Flowchart
│  │  └─ parts/       AssistantText, ThinkingState, LoadingState, TaskList,
│  │                  ContextGauge, MiscParts, useSmoothText
│  ├─ sessions/       SessionsSection, groupByDate, timeAgo
│  ├─ projects/       SidebarSections
│  ├─ terminal/       TerminalPanel
│  ├─ connection/     Banners
│  └─ …               (matches the relay's surface names)
└─ lib/               agent/{api,sse,store,reducer,protocol}, useShortcuts, utils
```

**Rule:** a component that renders agent data lives under `features/<surface>/`,
next to the reducer contract it serves. `components/` holds only the shell and the
vendored shadcn set. Tests sit beside their subject as `*.test.ts`.

## 5. Inventory as of 2026-09-29

### App bar — `features/chat/ChatAppBar.tsx`
Breadcrumb `session · project` on the left; live turn status and session actions
(close) on the right, always visible. The terminal toggle lives here in `actions`.

### Sidebar — `features/projects/SidebarSections.tsx`, `features/sessions/SessionsSection.tsx`
Project rows (name, truncated path, remove / open in Finder) and the session list
grouped by date (`groupByDate`: Today / Yesterday / This week / Older) with
relative timestamps (`timeAgo`) and the project name under each title. Both
grouping helpers are pure functions with their own tests. A busy session shows a
`Loader2` spinner on its row; **idle and error have no per-row icon** — adapter
lifecycle is reported by the banner strip (§ Banners), not by the sidebar.

### Transcript — `features/chat/Transcript.tsx`
One scroll column, `role="log"`, pin-to-bottom with a jump-to-latest pill
(`hooks/usePinnedScroll.ts`), parts in server order, edge fades, and the pixel-grid
loader while the agent works. With no session open the same column shows the
general empty state (`GeneralEmptyState` in `App.tsx`). A `retry` status renders
the retry banner inline instead of the loader.

### Message + parts
- `Message.tsx` renders both roles and maps one assistant message to its parts.
- `AssistantText.tsx` — streamed words resolve one by one behind a caret
  (`.caret-stream`, 55ms reveal), then settles to copyable text with an actions
  row; a sources toggle with avatar stack when the turn used web tools.
- `ThinkingState.tsx` — the agent trace as three row kinds (Reasoning / Search /
  Coding), header shimmering while any row runs, auto-expanding, then settling to
  a summary that stays manually expandable.
- `LoadingState.tsx` — pixel-grid loader: cells light in a chevron wavefront or
  orbiting comet via per-cell `animation-delay`; shimmer sweeps the label; reduced
  motion freezes it to the dim state while the elapsed timer keeps ticking.
- `TaskList.tsx` — agent subtask/task list.
- `MiscParts.tsx` — error blocks, subagent pills and the remaining small parts.
- `ContextGauge.tsx` — SVG ring on the composer's right; fills as the context
  window fills, tertiary → warning → danger, percentage centered. With no known
  window the ring is empty and the label falls back to a bare token count.

### Markdown — `features/chat/markdown/`
- `MarkdownView.tsx` routes fences: ` ```lang ` and ` ```diff ` to `CodePanel`,
  ` ```circulo-flow ` to `Flowchart`, everything else to prose.
- `CodePanel.tsx` — one editor-style panel for both: line-numbered code with light
  syntax coloring, or a unified diff with old/new gutters, accent bar, row tint
  and word-level add/del highlights. Copy + wrap.
- `Flowchart.tsx` — a workflow block rendered on a dotted canvas; JSON shape
  validated by `parseFlowBlock` (`nodes[{id,row,x,w,kind,hue,title,caption,
  condition}]`, `edges[{from,to}]`), both covered by tests.

### Composer — `features/chat/Composer.tsx`, `richInput.ts`
A `contenteditable` rich editor (not a textarea) so `@`-mentions and `/` commands
can sit **inline as colored tags** rather than as text. The placeholder is
CSS-generated (`.composer-rich:empty::before`) because the editor's children are
browser-owned and a React node would fight the DOM. Enter sends, Shift+Enter
newlines. Below it: model picker (one tab per project backend), branch chip, send/
stop pair, and the `ContextGauge`.

### Terminal — `features/terminal/TerminalPanel.tsx`
Project-scoped shells in a **separate card below the chat surface** (not inside
it: transcript and composer share one `bg-bg-main` card, and the terminal card is
a sibling that animates its own `max-height` to 240px). Tabs open/close in place.
I/O rides SSE (base64 chunks) plus write/resize POSTs, so it is transport-agnostic
to the provider and does not go through the chat protocol.

### Banners — `features/connection/Banners.tsx`
Adapter lifecycle as a **strip above the workspace**: `starting`, `stopped` and
`error` with the underlying cause (missing binary, port taken, crash). A running
project renders nothing. Provider-aware copy (`omp` vs `OpenCode`). The strip never
unmounts the chat — an earlier behavior replaced the transcript with the banner,
which read as "the chat left and came back".

## 6. Motion

Keyframes live in `index.css` and are used by the parts above: `pixel-on`,
`shimmer-text`, `pop-in`, `phrase-in`, `fade-in`, `fade-up`, `spin`. The Paper
bottom glow is a `linear-gradient` with an `oklab` stop and an sRGB fallback for
older webviews.

**`prefers-reduced-motion: reduce` is honored in four places** — the CSS animation
reset, `useSmoothText`, `GlowPixels`, and the composer. It freezes the animation,
not the information: the elapsed timer keeps ticking because it is a JS interval,
not a CSS animation.

## 7. Not built (v0)

Stated here so a reader does not read this doc as a promise:

- No light theme (§1).
- No ↑↓/Enter navigation of the session list — only the composer's own `@` and `/`
  menus take arrow keys ([ux.md](ux.md) §8).
- No toasts (`Sonner`) and no mobile-style sheet sidebar. Errors render in place.
- Sidebar has no collapse-to-icon rail; it slides to zero width.
