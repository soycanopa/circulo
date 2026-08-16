## Context

See `proposal.md`. The composer already reloads persisted parts after POST. Fake adapter turns include Markdown-ish text, a task list, and a tool call with a unified diff. `circulo-markdown` is an empty crate.

## Goals / Non-Goals

**Goals:**

- Parse Markdown and unified diffs in `circulo-markdown` without GPUI.
- Render those models in the app as part-specific views.
- Unit-test parse, context extraction, and task ordering.

**Non-Goals:**

- Incremental parse during stream (no SSE paint yet).
- Clickable OS link opening.
- Syntax highlighting inside code fences.

## Decisions

### 1. Parse outside GPUI

`circulo-markdown` depends on `pulldown-cmark` and returns `Block` / `Inline` / `DiffLine`. The app maps those to GPUI. Keeps tests fast and crate boundaries clean.

### 2. Question uses the unknown-part fallback

UX-UI §4.6 already requires a safe fallback. §4.10 QuestionCard stays out. Same locale key for Question and future unknown tags.

### 3. Tool cards start collapsed

Collapsed: name, status text, context line. Click toggles expansion (input JSON + output). Matches “nace compacto”.

### 4. Human name is the tool id with underscores as spaces

`edit_file` → `edit file`. Status copy is localized (`Ready`, `In progress`, …). Avoids inventing a verb table.

### 5. Diff prefers stored `diff`, else `new_content`

Parse unified hunks when `diff` is present. If only `new_content` exists, show those lines as additions.

## Risks / Trade-offs

- [GPUI has no Markdown widget] → custom block renderer; tables stay simple flex rows.
- [Incomplete Markdown can look odd] → acceptable until streaming; parser still must not panic.

## Migration Plan

None.

## Open Questions

None.
