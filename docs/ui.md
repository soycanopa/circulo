# Circulo — UI Specification (v1.0)

- **Status:** Draft v1.0 (2026-11-20)
- **Related:** [UX](ux.md) · [PRD](prd.md)
- **Design source:** Match the visual design and layout of [Waku](https://github.com/egoist/waku). Later personal adjustments expected. Visual language and interaction patterns only — no code copy (GPL-3.0).
- **Implementation:** MyGo native UI (`ui` package), including `ui/markdown` for the transcript. The window is not a web frontend. A later Browser tab may use `Window.NewWebView` under the native view.

## 1. Design language (Waku graphite)

- Neutral **graphite** palette. Color is reserved for meaning (status, selection, streaming, gauges).
- Accent: coral / warm tone (Waku style) for primary actions and highlights.
- Support dark, light, and system appearance.
- macOS: sidebar vibrancy / blur where practical.
- High information density, calm, keyboard-first.
- Content (transcript) selectable; chrome not.

## 2. Layout geometry (Waku-style)

- Custom titlebar / app bar (session + project context, global actions).
- Resizable left sidebar (projects + sessions).
- Main center column: transcript (pin-to-bottom) + composer fixed at bottom.
- Optional right panel with tabs (Files / Diff / Terminal / Browser / Editor).
- Permission / question cards stack directly above the composer.
- Minimum window size similar to Waku (~980×680).

## 3. Key surfaces (match Waku)

### Sidebar
- Project list with status indicators.
- Session list with grouping (date, folders), pinning, archiving, drag-reorder, context menus.
- Search / filter controls in header.

### Transcript
- Virtualized list for long conversations.
- Parts in server order: reasoning (collapsible), tool calls / activities, markdown text, usage footer.
- Streaming text with caret or smooth reveal (respect reduced motion).
- Navigation rail / minimap for jumps between turns.
- Pin-to-bottom + “jump to latest” pill.

### Composer
- Multi-line input with @-mentions and /commands.
- Model + reasoning effort + access mode pickers.
- Send / Stop / Steer buttons.
- Context / usage gauge.

### Right panel
- Tabbed: Files (worktree), Diff, Terminal (PTY), Browser (optional), File Editor.
- Native integration where MyGo allows it.

### Other
- Settings (searchable sidebar + content).
- Checkpoint / rewind UI.
- Command palette.

## 4. Tokens (starting point — graphite)

Approximate starting values (to be refined to match Waku screenshots):

| Role | Dark (approx) | Notes |
|---|---|---|
| Canvas | `#1A1A1A` | Main background |
| Sidebar | transparent / blur | macOS vibrancy |
| Text | `#E2E2E2` | Primary |
| Accent | `#E2795B` | Coral |
| Borders | subtle gray | |

Exact tokens will be extracted from Waku screenshots / inspection and then adjusted by the owner.

## 5. Motion

Subtle, purposeful. Honor system reduced-motion. No decorative blink unless agent is working.