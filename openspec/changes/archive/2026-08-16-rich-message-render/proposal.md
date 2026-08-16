## Why

The composer can send a turn, but the transcript is still a flat dump. Marketers and PMs need Markdown, tool cards, diffs, and task lists to trust what the agent did (PRD-CHT-03, PRD-CHT-05, PRD-CHT-06, PRD-CHT-07).

## What Changes

- Parse `MessagePart::Text` as Markdown (headings, lists, tables, blockquote, code, links, emphasis).
- Render `ToolCall` as an expandable card: human name, status, one-line context, input, and output.
- Render `ToolOutput::Diff` as a file path plus +/- lines with internal scroll.
- Render `TaskList` as a non-interactive list ordered by `order`.
- Unknown or `Question` parts use a safe fallback string. No Question UI (PRD-CHT-10).

## Capabilities

### New Capabilities

- `rich-message-render`: Part-aware chat rendering for text, tools, diffs, and tasks.

### Modified Capabilities

- (none)

## Non-goals

- No live SSE paint (still reload after POST).
- No interactive task checkboxes.
- No QuestionCard.
- No cancel.

## Impact

- `circulo-markdown`: parse Markdown and unified diffs (no GPUI).
- `circulo-app`: replace the plain message list with part renderers.
- Locale keys for tool/task status and the unsupported-part fallback.

## Open questions (not resolved here)

None that block this change. Question parts use the same unknown-part fallback as UX-UI §4.6.
