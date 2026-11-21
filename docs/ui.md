# Circulo — UI Specification (v1.0)

- **Status:** Draft v1.0 (2026-11-21)
- **Related:** [UX](ux.md) · [PRD](prd.md)
- **Design source:** the published Waku screenshots (Waku 0.1.20, [`app-screenshot-dark.png`](https://waku.sh/app-screenshot-dark.png) and [`app-screenshot-light.png`](https://waku.sh/app-screenshot-light.png)). We replicate the visual design and the layout: geometry, palette, type steps and component order are measured from those captures and reimplemented in GPUI.
- **Rule:** Waku is GPL-3.0. We copy the *look*, never a file, a function or a type. `AGENTS.md` holds the rule; this document holds the measurements.
- **Implementation:** native GPUI. One window, no embedded web view.

## 1. Design language (Waku graphite)

- Neutral **graphite** palette. Colour is reserved for meaning (status, selection, streaming, gauges); the interface itself is monochrome.
- Accent: coral / warm tone, used for primary actions and status once the core can report them.
- Dark, light and system appearance. The palette below is the measured pair.
- macOS: the window is transparent to the desktop with a blurred background; the sidebar is a translucent material, the main column paints an opaque canvas.
- High information density, calm, keyboard-first.
- Transcript content is selectable; chrome is not.

## 2. Layout geometry (measured, in points)

| Region | Value |
|---|---|
| Window, default | 1000 × 740 |
| Window, minimum | 980 × 680 |
| Sidebar | 252 wide, full height, translucent |
| Sidebar top row (window controls) | 46 high; traffic lights at 20 / 18; controls start at 92 |
| Sidebar horizontal inset (list) | 12, rows inset 12 more |
| Task row | radius 8, padding 12 × 8, two lines with a 3 gap |
| Main header row | 46 high, 24 horizontal inset |
| Transcript | 34 horizontal padding, no narrower reading column |
| User bubble | max 520 wide, radius 12, padding 15 × 9, right-aligned |
| Composer | 24 horizontal inset, radius 14, 13 padding, 12 between input and controls |
| Picker | 26 high, radius 6 |
| Send | 28 circle |

The transcript scrolls under the composer; the composer and its footer stay at the bottom of the main column.

## 3. Tokens (measured from the reference)

Dark:

| Role | Value | Where it is used |
|---|---|---|
| Canvas | `#141414` | Transcript and composer background |
| Sidebar | translucent graphite (reads `#1A1E20` over a blue wallpaper) | Sidebar material over the window blur |
| Selection | `#FFFFFF` at 7% | Selected task row |
| Hover | `#FFFFFF` at 4% | Rows, icon buttons, pickers |
| Surface | `#191919` | Composer |
| Border | `#FFFFFF` at 8% | Composer outline |
| Chip | `#242424` | User bubble, inline code, send button |
| Text | `#DCDCDC` | Titles, transcript body |
| Text muted | `#9A9A9A` | Picker labels, group labels |
| Text faint | `#6F6F6F` | Metadata, times, icons |
| Hairline | `#FFFFFF` at 10% | Worked divider |
| Accent | `#E2795B` | Status and the active send state |

Light:

| Role | Value |
|---|---|
| Canvas | `#F5F4F5` |
| Sidebar | translucent light material (reads `#E9EFF1` over the same wallpaper) |
| Selection | `#000000` at 5% |
| Hover | `#000000` at 3% |
| Surface | `#FFFFFF` |
| Border | `#000000` at 10% |
| Chip | `#E6E5E6` |
| Text | `#1B1B1B` |
| Text muted | `#6B6B6B` |
| Text faint | `#8E8E8E` |
| Hairline | `#000000` at 8% |
| Accent | `#D9663F` |

Type: system font. 13 for titles and composer, 12 for pickers, 11.5 for group labels, metadata and times. Inline code is one step smaller in a monospace face.

## 4. Surfaces and components (replication checklist)

### Sidebar
- [x] Window controls row: traffic lights, sidebar toggle, back and forward.
- [x] `New Task` and `Search` rows with icons.
- [x] Date-grouped task list. Group label with the folder affordance on the group that holds the open task.
- [x] Task row: one-line title, project with folder icon, relative time right-aligned, selected and hover states.
- [x] Settings at the bottom.
- [ ] Search field, context menus, drag-reorder, pinning and archiving.

### Transcript
- [x] Header with the task title and the right-panel toggle.
- [x] Right-aligned user bubble.
- [x] `Worked for …` divider with hairline rules and a disclosure chevron.
- [x] Assistant blocks: heading, paragraph, bullet, inline code, bold.
- [ ] Reasoning and tool-activity rows (collapsed by default).
- [ ] Virtualized list, pin-to-bottom and the jump-to-latest pill.
- [ ] Streaming caret and reduced-motion behaviour.

### Composer
- [x] Input surface with placeholder.
- [x] Shared controls: model, reasoning effort, access mode, agent mode; send button.
- [x] Footer: project, `Local`, branch, activity indicator.
- [ ] Real menus, queue and steer affordances, usage gauge, attachments.

### Missing for v1.0
Permission cards, checkpoint and rewind UI, the right panel tabs (files, diff, terminal, browser, editor), settings, command palette, empty state.

## 5. Motion

Subtle, purposeful. Honor system reduced-motion. No decorative blink unless an agent is working. Streaming text reveals without a caret when reduced motion is on.

## 6. How we keep it faithful

Every change to this screen is checked against the reference captures side by side, at the same window size and appearance, before it merges. Deviations are deliberate and recorded here: layout adjustments by the owner later, never accidental drift.
