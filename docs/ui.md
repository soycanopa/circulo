# circuloGo — UI Specification

- **Status:** Draft v0.1 (2026-09-02)
- **Related:** [UX](ux.md) (behavior) · [Flow](flow.md)
- **Design language:** calm, terminal-adjacent, dark-first. Fully neutral zinc
  palette — no accent hue (owner decision 2026-09-02: violet rejected); status
  colors are reserved for meaning only (green=running, amber=permission/retry,
  red=error). No decorative gradients.

## 1. Theme tokens (Tailwind v4 CSS variables, shadcn-compatible)

| Token | Dark (default) | Light |
|---|---|---|
| background | `zinc-950` | `white` |
| sidebar bg | `zinc-900` | `zinc-50` |
| card / composer | `zinc-900` | `white` (border `zinc-200`) |
| border | `zinc-800` | `zinc-200` |
| foreground / muted-fg | `zinc-100` / `zinc-400` | `zinc-900` / `zinc-500` |
| primary | `zinc-100` (dark) / `zinc-900` (light) | |
| code block bg | `zinc-900/60` + border | `zinc-50` + border |
| success / warning / danger | `emerald-500` / `amber-500` / `red-500` | same |
| font | UI: system sans (`Inter` optional); code: `JetBrains Mono`/`SF Mono` stack | |

Radii: `rounded-lg` cards, `rounded-md` chips. Shadows minimal (`shadow-sm` on
floating pill only). Spacing rhythm: 4/8/12/16.

## 2. Component inventory

```
components/
├─ layout/
│  ├─ AppShell            grid: sidebar | main; drag-resize handle (min 232, max 360)
│  └─ TitleBar            h-11, traffic-light inset (Mac titlebar hidden-inset style)
├─ sidebar/
│  ├─ Sidebar             collapse (w-72 ↔ w-12), sections below
│  ├─ NewChatButton      primary button, w-full
│  ├─ ProjectSwitcher     rows: status-dot, name, path (truncated); context: Remove,
│  │                      Open in Finder; "+ Add project" row
│  ├─ SessionList         group headers (Today/Yesterday/This week/Older), rows:
│  │                      title, tiny status icon (spinner/check/alert), hover ⋯ menu
│  └─ SessionRow          active = accent left rail; inline rename on double-click
├─ chat/
│  ├─ Transcript          scroll container, pin-to-bottom logic, JumpToLatest pill
│  ├─ MessageList         maps MessageRecord[] → rows
│  ├─ UserMessage         right-accent bubble + attachment chips
│  ├─ AssistantTurn       groups one assistant message: parts + TurnFooter
│  ├─ TurnFooter          "Worked for 42s · 17.5k tok · $0.03" (from step-finish agg)
│  ├─ ReasoningPart       Collapsible: header "Thinking · Ns", stream tail while live
│  ├─ TextPart            MarkdownView wrapper
│  ├─ ToolCard            per UX §4; ToolIcon by kind; state chip; expand=details
│  ├─ ToolGroup           folded cluster "Worked · N steps" for finished turns
│  ├─ PatchCard           Changed files + per-file UnifiedDiff (collapsed by default)
│  ├─ SubtaskPill         agent/subtask slim pill
│  ├─ ErrorBlock          session.error rendering (name + message, red border)
│  ├─ StatusStrip         retry banner / working timer
│  ├─ Composer            textarea (auto-grow ≤6 lines), SendOrStop button,
│  │                      ModelPicker + AgentPicker dropdowns, attachment chips
│  └─ PermissionCard      per UX §7
└─ ui/                    shadcn: Button, Input, Textarea, Dialog, DropdownMenu,
                          ContextMenu, Tooltip, ScrollArea, Badge, Collapsible,
                          Select, Separator, Sheet (mobile-ish sidebar), Sonner (toasts)
```

### shadcn mapping

| Need | shadcn component | Notes |
|---|---|---|
| Send/Stop, permission buttons | `Button` (variants) | primary = accent |
| Session/project menus | `ContextMenu` + `DropdownMenu` | rename/delete |
| Model/agent pickers | `DropdownMenu` with `Command`-style search | show provider sub-label |
| Rename dialog, delete confirm | `Dialog` (`AlertDialog` for delete) | |
| Tool output / diff scroll | `ScrollArea` | max-h-96 |
| ToolCard expand | `Collapsible` | animated height |
| Tool state chip | `Badge` (variants: outline/success/danger) | |
| Toasts (non-blocking errors) | `Sonner` | errors only; stream state stays in-place |
| Jump to latest | `Button` floating pill | `shadow-sm` |

## 3. Key mockups

### Composer + permission (busy with pending permission)

```
┌──────────────────────────────────────────────────────┐
│ ⚠ Permission · bash                                   │
│ Run `npm test -- --coverage` in ~/code/api  [▾ detail]│
│            [Allow once]  [Always allow]  [Deny]       │
├──────────────────────────────────────────────────────┤
│ model: GLM-5.3 ▾   agent: build ▾                     │
│ ┌──────────────────────────────────────────────────┐ │
│ │▎Now add a CI workflow that…                      │ │
│ └──────────────────────────────────────────────────┘ │
│                                            [ ■ Stop ] │
└──────────────────────────────────────────────────────┘
```

### ToolCard (expanded, completed)

```
┌──────────────────────────────────────────────────────┐
│ ⚙ bash  npm test -- --coverage              ✓ 2.1s ▾ │
│ ── output ────────────────────────────────────────── │
│ $ npm test                                           │
│ ✓ 14 tests passed (3.2s)                     [copy]  │
│ ── input ─────────────────────────────────────────── │
│ { "command": "npm test -- --coverage" }              │
└──────────────────────────────────────────────────────┘
```

### Reasoning (streaming, open)

```
┌──────────────────────────────────────────────────────┐
│ ✻ Thinking · 6s                                ▴     │
│ The failing tests are in auth/session.ts. The mock   │
│ clock isn't reset between cases, so retry assertions │
│ █ (cursor)                                           │
└──────────────────────────────────────────────────────┘
```
Collapsed after completion → `▸ Thinking · 12s` single row.

## 4. Markdown rendering spec (`MarkdownView`)

- `react-markdown` + `remark-gfm`; per-message `React.memo`; streaming message only
  re-renders its last text part.
- Code block component: header bar (language label, copy button → ✓ 2s), `<pre>` with
  `white-space: pre-wrap; overflow-wrap: anywhere` (waku-style wrapping, no horizontal
  scroll), mono font, token budget: no syntax highlighting lib in v0 — quiet mono keeps
  streaming cheap (highlighting is a v1 candidate behind the same component).
- Tables: bordered, header row muted; overflow-x auto *inside* the table container only.
- Links: accent underline, open in system browser via Wails Browser API (never in-webview
  navigation).
- Inline code: `code bg` token, rounded-sm.
- Headings in assistant messages: h1→h3 scaled down (start at text-base) to sit in chat.

## 5. Iconography

`lucide-react` (shadcn default). Tool icon map: bash→`SquareTerminal`, edit/write→`FilePen`,
read→`FileText`, search/grep→`Search`, glob/list→`FolderTree`, webfetch→`Globe`,
task/subagent→`GitBranch`, todo→`ListTodo`, default→`Wrench`. State chips: pending `CircleDashed`,
running `Loader2` (spin), completed `Check`, error `X`.

## 6. Window

- Default 1280×800, min 960×640; dark backdrop color to avoid white flash
  (`BackgroundColour` = zinc-950); macOS `TitleBar: HiddenInset` + translucent backdrop,
  draggable titlebar region h-11.
- Single instance (Wails `SingleInstance`): second launch focuses existing window.
