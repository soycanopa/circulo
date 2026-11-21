# Circulo — UI Specification (v1.0)

- **Status:** Draft v1.0 (2026-11-21)
- **Related:** [UX](ux.md) · [PRD](prd.md)
- **Design source:** Waku 0.1.20. The ground truth is a capture of the app running on this machine (1380 × 880, dark appearance); the published screenshots ([`app-screenshot-dark.png`](https://waku.sh/app-screenshot-dark.png), [`app-screenshot-light.png`](https://waku.sh/app-screenshot-light.png)) fill in states the empty app does not show, and are the 2× scale reference for spacing.
- **Rule:** Waku is GPL-3.0. We copy the *look*, never a file, a function or a type. `AGENTS.md` holds the rule; this document holds the measurements.
- **Implementation:** native GPUI. One window, no embedded web view.

## 1. Design language (Waku graphite)

- Neutral **graphite** palette. Colour is reserved for meaning (status, selection, streaming, gauges); the interface itself is monochrome.
- Accent: coral / warm tone, used for primary actions and status once the core can report them.
- Dark, light and system appearance. Dark is measured; light is still an approximation (§ 3).
- High information density, calm, keyboard-first.
- Transcript content is selectable; chrome is not.

**No vibrancy.** The marketing screenshots show a sidebar tinted by a blue wallpaper, which looks like a vibrancy material. It is not what the app does: the live capture reads a flat `#252525` over a `#1A1A1A` canvas. GPUI exposes `WindowBackgroundAppearance::Blurred`, which inserts an `NSVisualEffectView`; on macOS 27 it tints nothing here, and Waku does not rely on it. The sidebar is painted opaque, as the app does it.

## 2. Layout geometry (measured, in points)

| Region | Value |
|---|---|
| Window, default | 1000 × 740 |
| Window, minimum | 980 × 680 |
| Sidebar | 252 wide, full height, opaque |
| Sidebar top row (window controls) | 46 high; traffic lights at 16 / 18; controls start at 82 |
| Sidebar list inset | 4; group label inset 14; row inset 14 |
| Task row | 51 high, radius 8, padding 14 × 8, two lines with a 3 gap |
| Main header row | 46 high, 24 horizontal inset |
| Transcript | 34 horizontal padding, no narrower reading column |
| User bubble | max 520 wide, radius 12, padding 15 × 9, right-aligned |
| Composer | 24 horizontal inset, radius 14, 13 padding, 12 between input and controls |
| Picker | 26 high, radius 6 |
| Send | 28 circle |

The transcript scrolls under the composer; the composer and its footer stay at the bottom of the main column.

## 3. Tokens (measured from the live app)

Dark:

| Role | Value | Where it is used |
|---|---|---|
| Canvas | `#1A1A1A` | Transcript background |
| Sidebar | `#252525` | Sidebar surface |
| Selection | `#FFFFFF` at 4% | Selected task row |
| Hover | `#FFFFFF` at 3% | Rows, icon buttons, pickers |
| Surface | `#1F1F1F` | Composer |
| Border | `#FFFFFF` at 8% | Composer outline |
| Chip | `#2A2A2A` | User bubble, inline code, send button |
| Text | `#E2E2E2` | Titles, transcript body |
| Text muted | `#A3A3A3` | Actions, group labels, picker labels |
| Text faint | `#747474` | Metadata, times, icons |
| Hairline | `#FFFFFF` at 10% | Worked divider |
| Accent | `#E2795B` | Status and the active send state |

`Surface`, `Border`, `Chip`, `Selection` and `Hairline` are derived from the offsets between canvas and component in the 2× capture, applied to the live palette. The empty app does not show them.

Light:

| Role | Value |
|---|---|
| Canvas | `#F5F5F5` |
| Sidebar | `#ECECEC` |
| Selection | `#000000` at 4% |
| Hover | `#000000` at 2% |
| Surface | `#FFFFFF` |
| Border | `#000000` at 8% |
| Chip | `#E9E9E9` |
| Text | `#1B1B1B` |
| Text muted | `#6F6F6F` |
| Text faint | `#9A9A9A` |
| Hairline | `#000000` at 8% |
| Accent | `#D9663F` |

Light has not been captured from the live app yet; these values follow the published light screenshot and are corrected the first time we take a light capture.

Type: system font. 13 for titles and composer, 12 for pickers, 11.5 for group labels, metadata and times. Task rows use 17 / 15 line height, transcript blocks 20. Inline code is one step smaller in a monospace face.

## 4. Surfaces and components (replication checklist)

### Sidebar
- [x] Window controls row: traffic lights, sidebar toggle, back and forward.
- [x] `New Task` and `Search` rows with icons.
- [x] Date-grouped task list. Group label with the folder affordance on the group that holds the open task.
- [x] Task row: one-line title, project with folder icon, relative time right-aligned, selected and hover states.
- [x] Settings at the bottom.
- [x] `New Task` shows the empty state; a task row opens the conversation. Routing only: no project is created yet.
- [ ] Group filter affordance (the live app shows one next to the folder when a group is active).
- [ ] Search field, context menus, drag-reorder, pinning and archiving.

### Transcript
- [x] Header with the task title and the right-panel toggle.
- [x] Right-aligned user bubble.
- [x] `Worked for …` divider with hairline rules and a disclosure chevron.
- [x] Assistant blocks: heading, paragraph, bullet, inline code, bold.
- [x] Live parts from the daemon: reasoning row, tool row that updates from running to done, streamed text with paragraphs.
- [x] Collapsed reasoning and tool rows: icon, label, target, and a disclosure chevron or a spinner.
- [x] Empty state: coral mark, title, subtitle, primary button and the quiet way out.
- [ ] Expanded reasoning, tool output and live status; the rows are static.
- [ ] Virtualized list, pin-to-bottom and the jump-to-latest pill.
- [ ] Streaming caret and reduced-motion behaviour.

### Composer
- [x] Input surface with placeholder.
- [x] Shared controls: model, reasoning effort, access mode, agent mode; send button.
- [x] Footer: project, `Local`, branch, activity indicator.
- [ ] Provider glyph in the model picker.
- [ ] Real menus, queue and steer affordances, usage gauge, attachments.
- [ ] The empty-state folder picker and the “don’t work in a project” action (both need the core).

### Missing for v1.0
Permission cards, checkpoint and rewind UI, the right panel tabs (files, diff, terminal, browser, editor), settings, command palette.

## 5. Motion

Subtle, purposeful. Honor system reduced-motion. No decorative blink unless an agent is working. Streaming text reveals without a caret when reduced motion is on.

## 6. How we check fidelity

1. Run the app: `cargo run -p circulo`. It opens at 1000 × 740, the size the reference measurements use.
2. Find the window id and capture it alone:

   ```sh
   # window id for the process named "circulo", then
   screencapture -l<id> -x -o capture.png
   ```

3. Compare side by side with the Waku capture at the same size, and with the 2× screenshot for states the app does not show (halve its pixel values).
4. Any deliberate deviation is written in this file. No accidental drift.
