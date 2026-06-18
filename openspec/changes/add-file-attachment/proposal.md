## Why

Users cannot attach text files (`.md`, `.txt`, etc.) or easily find the attach button in the chat interface. The existing `AttachButton` only accepts images and PDFs, and is completely absent from the new-chat screen. Users need a visible, accessible way to attach all common file types—including images, PDFs, markdown, and plain text—in both the new-chat and in-session chat views.

## What Changes

- Add a visible attach button to the **new-chat screen** (`new-chat.tsx`) — currently missing entirely
- Extend accepted file types to include text-based formats: `.txt`, `.md`, `.json`, `.csv`, `.ts`, `.js`, `.tsx`, `.jsx`, `.log`, `.yaml`, `.yml`, `.toml`, `.env`, `.html`, `.css`, `.xml`, `.py`, `.rb`, `.go`, `.rs`, `.java`, `.c`, `.h`, `.cpp`, and `.sql`
- Ensure the attach button is consistently positioned **before** the agent/model/variant toolbar in both views
- Increase max file size from 10MB to 25MB to accommodate larger text/code files
- Add visual feedback: show the button prominently as an icon-only button (not hidden behind tooltip-only discovery)

## Capabilities

### New Capabilities
- `file-attachment`: Allow users to attach images, PDFs, markdown, plain text, and common source code files to chat messages via a visible toolbar button in both new-chat and in-session views. Includes drag-and-drop and clipboard paste for the same file types.

### Modified Capabilities
<!-- No existing capability requirements are changing -->

## Impact

- **Affected components**: `apps/desktop/src/renderer/components/chat/chat-view.tsx` (add/refine AttachButton, extend accept types), `apps/desktop/src/renderer/components/new-chat.tsx` (add AttachButton, extend accept types)
- **Affected UI library**: `packages/ui/src/components/ai-elements/prompt-input.tsx` (max file size default, accept type handling — no logic changes needed, accept is already a prop)
- **No API changes**: File sending via `use-server.ts` `sendPrompt` already supports arbitrary `FileAttachment[]`
- **No backend changes**: OpenCode SDK `promptAsync` already handles `FilePartInput`
- **No breaking changes**: Existing image/PDF attachment behavior is preserved
