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
| Picker menu | 360 wide, 320 tall at most, radius 10, scroll, surface + border + shadow |
| Effort editor | 160 wide, radius 10, surface + border + shadow; floats beside the menu, level with its row |
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

Type: system font. 13 for titles and composer, 12 for pickers, 11.5 for group labels, metadata and times. Task rows use 17 / 15 line height, transcript blocks 20. Code is a monospace face (`Menlo`): inline runs keep the body size, fenced blocks drop to 12 / 18 on a chip surface.

## 4. Surfaces and components (replication checklist)

### Sidebar
- [x] Window controls row: traffic lights, sidebar toggle, back and forward.
- [x] `New Task` and `Search` rows with icons.
- [x] Date-grouped session list from the database: Today / Yesterday / This Month / month name, title from the first prompt, project folder, relative time, selection. The folder affordance marks the group that holds the open session. A row opens its history; `New Task` starts a session.
- [x] Task row: one-line title, project with folder icon, relative time right-aligned, selected and hover states.
- [x] Settings at the bottom.
- [ ] The no-project onboarding screen was removed when sessions became real: `New Task` starts an empty session. Projects will bring it back with a folder picker.
- [ ] Group filter affordance (the live app shows one next to the folder when a group is active).
- [ ] Search field, context menus, drag-reorder, pinning and archiving.

### Transcript
- [x] Header with the task title and the right-panel toggle.
- [x] Right-aligned user bubble.
- [x] `Worked for …` divider with hairline rules and a disclosure chevron.
- [x] Assistant blocks: heading, paragraph, bullet, inline code, bold.
- [x] Live parts from the daemon: reasoning row, tool row that updates from running to done, streamed text with paragraphs.
- [x] Rendered through a GPUI `List` of turn rows, as the reference does, not a plain flex column.

### How the transcript is rendered

Two rules, learned the hard way and worth keeping:

1. **The scroll container is a `List`, not a flex column.** A `div` that is both the scrolling viewport and the column measures its children against the viewport and they shrink into each other. The list virtualizes, owns the offset and gives every row a stable id (`row-{index}`, `row-{index}-segment-{index}`).
2. **Every row is `w_full` and does not shrink.** A row measured at its minimum width reports a shorter height than it paints, so the next row lands on top of it. Turn rows mark `flex_shrink_0` on each block.

Two more, learned from a real OpenCode turn rather than from the demo one:

3. **No fixed heights on a row that carries provider text.** The reasoning and tool rows use `min_h`, never `h`: a row is allowed to grow with its content and push the rows below it. A fixed height only works while the content fits.
4. **A collapsed row's text is flattened to one line before it is painted.** GPUI splits any text element on `\n` regardless of `white_space`, so a reasoning part with paragraphs, or a shell command written on two lines, painted every line from one 26 px box and straight over the next rows. `transcript::flatten` joins whitespace runs with single spaces; `whitespace_nowrap` and `text_ellipsis` then do their job.

Streaming changes a row's height without changing the row count; the list is reset while a turn runs so it measures the turn again instead of keeping last frame's height.

The list is **bottom-anchored** (`ListAlignment::Bottom`). That is what makes a streaming turn follow its newest row: a reset clears the logical offset, and a bottom-anchored list with no offset sits at the end. Once the reader scrolls up the offset sticks, and a circular **jump-to-latest** button appears above the composer; it scrolls to `item_count` (one past the last row, which is how a bottom-anchored list expresses "the end") and disappears once the offset is back at the bottom.

### How markdown is rendered

Agent text is CommonMark plus the GitHub extensions agents actually write (tables, strikethrough, task lists), and it streams, so the renderer has to accept partial input. `crates/ui/src/markdown.rs` parses with `pulldown-cmark` (no `html` or `getopts` features) into `Block`s — paragraph, heading, list item, quote, code, table, rule — each holding `Span`s with inline flags (bold, italic, code, strike, link).

- **Drawing uses `StyledText` with one `TextRun` per span**, not `with_default_highlights`. Highlights can change weight, colour and decoration but not the font family, and inline code needs the monospace family. Each run carries its own `Font`, colour and background.
- **Inline code is a mono run on the `chip` colour**; the paper has no padding inside a run, so the chip hugs the glyphs.
- **Lists** use a fixed 18 px marker column and indent one level by 18 px, so wrapped lines stay aligned with the text above them. Items of the same list sit 4 px apart; every other block 9 px.
- **Fenced blocks** paint their own chip surface at 12 / 18 in `Menlo`; GPUI splits a text element on `\n`, so the block needs no per-line layout.
- **Footnotes** parse as `[^label]` references. A reference paints as a superscript digit in the accent colour (`¹`), numbered in reference order; a bare `[1]` was tried first and broke across a line, because GPUI treats a closing bracket as a break opportunity. Definitions move to the end of the message as a numbered note list, so a definition written mid-document still reads in the right place. A reference with no definition stays literal `[^label]`, the way GitHub renders it.
- **The parse is cached per segment.** `markdown::Cached` keeps the blocks until the text grows, so a finished message is not re-parsed on every re-render (scroll, hover, resize). The cache lives on the streamed segment, so it cannot leak across sessions; the text length is the version, which is enough because streamed text only appends.
- **Tables** are rows of cells with a 1 px hairline above and between rows, no vertical rules. Each column takes a share of the width in proportion to the longest text it holds (floored at 6 characters, so a narrow "N" column stays readable), and the delimiter row's alignment is honoured through `text_align`.
- **Partial input never fails**: an unclosed fence is already a code block, an unclosed `**` stays literal until its marker arrives. Footnotes, math and raw HTML are not rendered yet (HTML is dropped); links are coloured but not clickable, because a click would have to hit-test a byte range inside a text layout this renderer rebuilds every frame.
- [x] Collapsed reasoning and tool rows: icon, label, target, and a disclosure chevron or a spinner.
- [x] Question card: the agent's message, a label per field, choice rows with the chosen one tinted, free-text fields with the composer's input and caret, Submit disabled until every required field is answered, and Decline and Cancel as the two ways out.
- [x] Permission card between the transcript and the composer: warning glyph, the agent's title, the command or path in full, and one button per option the agent offered. Enter allows, Escape rejects. Design in `docs/permissions.md`.
- [x] Empty state: coral mark, title, subtitle, primary button and the quiet way out.
- [ ] Expanded reasoning, tool output and live status; the rows are static.
- [x] Bottom-anchored list that follows the stream, and the jump-to-latest button.
- [ ] Streaming caret and reduced-motion behaviour.

### Composer
- [x] Editable input: buffer, block caret, insertion at the caret, backspace/delete, left/right/home/end. Enter sends, Shift+Enter breaks the line; the field grows to four lines and then scrolls. `CIRCULO_PROMPT` seeds a draft.
- [x] Input surface with placeholder.
- [x] Shared controls: model, reasoning effort, access mode, agent mode; send button.
- [x] Model, effort and mode pickers fed by the daemon's `Options`: click opens the agent's choices with the current one marked, Escape closes, choosing sends `SetOption` and the refreshed list redraws the labels. Access stays a mock label.
- [x] Compact menus (360 × 320, scrolling): the model menu has two tabs — **Favorites** (star icon) and the agent's name (layers icon), the star on every row, starred models stored through the settings wire and leading the menu when they exist — and the agent's list grouped under one heading per provider (`DeepSeek`, `OpenCode`, …), with the vendor tag stripped from each name by the driver. Effort and mode share the container without tabs.
- [x] Footer: project, `Local`, branch, activity indicator.
- [x] Provider glyph in the model picker: the tab carries the agent's name and the layers icon.
- [x] Effort editor beside the model menu: hovered rows gain an `Edit` affordance, and clicking it floats the agent's effort choices level with that row — one editor, moved between rows. Picking a value applies the model with it in one decision (two `SetOption`s, the model only when it changed); Escape or a closed menu dismisses it. The editor anchors to the row's measured position and flips upward near the window's bottom edge. `CIRCULO_OPEN_PICKER` and `CIRCULO_OPEN_EDIT` open both without a click for testing.
- [x] Stop while a turn runs: the send circle becomes a stop square and Escape stops too (menus and cards keep their own Escape). The daemon cancels the turn's cards, the provider aborts, and the transcript closes it as `Stopped` without an error banner.
- [ ] Queue and steer affordances, usage gauge, attachments.
- [ ] The empty-state folder picker and the “don’t work in a project” action (both need the core).
- [ ] Selection, undo, IME composition, and paste: the buffer is deliberately single-carets-at-the-end-of-a-line for now (`crates/ui/src/input.rs`).

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
