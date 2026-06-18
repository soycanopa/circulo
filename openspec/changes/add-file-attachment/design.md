## Context

Circulo already has a file attachment infrastructure:
- **UI library** (`packages/ui/src/components/ai-elements/prompt-input.tsx`): `PromptInput` accepts `accept`, `multiple`, `maxFileSize` props. Handles drag-and-drop, clipboard paste, and file validation via `matchesAccept`. Exposes `usePromptInputAttachments()` hook for file management.
- **Chat view** (`chat-view.tsx`): Has an `AttachButton` component calling `attachments.openFileDialog()`, rendered in `<PromptInputTools>` inside `<PromptInputFooter>`. Currently only accepts `image/*` and `application/pdf`.
- **New-chat view** (`new-chat.tsx`): Has `<PromptInput>` with the same restrictive `accept` prop but **no `AttachButton`** rendered.
- **File sending** (`use-server.ts`): `sendPrompt` already handles arbitrary `FileAttachment[]` — converts to `FilePartInput` for the OpenCode SDK.
- **Attachment preview** (`prompt-attachments.tsx`): Shows image thumbnails or file icons with remove buttons and model capability warnings.

The gap is discoverability and file type coverage: users can't easily find the attach button (missing from new-chat entirely, and the `+` icon in chat-view is subtle), and text files are excluded from the accept filter.

## Goals / Non-Goals

**Goals:**
- Add `AttachButton` to the new-chat screen, matching the pattern used in chat-view
- Extend `accept` in both chat-view and new-chat to include text/code file types
- Ensure consistent button placement: attach button always appears left of the agent/model toolbar in `<PromptInputTools>`
- Increase max file size from 10MB to 25MB for both views

**Non-Goals:**
- No new UI library components — reuse existing `PromptInputButton`, `PlusIcon`, and `usePromptInputAttachments`
- No changes to file sending pipeline — `sendPrompt` already supports arbitrary `FileUIPart[]`
- No changes to drag-and-drop or paste behavior — those already work via `<PromptInput>`
- No support for binary files beyond images and PDFs (e.g., no `.zip`, `.tar`, compiled binaries)
- No auto-conversion of attached code files to text context (file parts are sent as-is; the OpenCode server determines how to process them)

## Decisions

### 1. Reuse existing `AttachButton` component rather than creating a new one

**Rationale**: The `AttachButton` already exists in `chat-view.tsx` as a local function component. Extract it to a shared module so both `chat-view.tsx` and `new-chat.tsx` use the same consistent implementation.

**Alternative considered**: Duplicating the button code in new-chat. Rejected — violates DRY and risks divergent behavior.

### 2. Accept text/code files via MIME type + extension fallback

**Rationale**: The `matchesAccept` function in `prompt-input.tsx` checks `f.type` (MIME). Text/code files from the OS file picker typically have an empty MIME (`""`). The `PromptInput`'s `accept` string is used both for the `<input type="file">` native filter AND the `matchesAccept` function in `prompt-input.tsx`. 

For the native `<input accept>`, we can add extension patterns like `.md,.txt,.json`. For `matchesAccept`, we need to also allow files with empty MIME (undetermined type), since many text files report `""` as their MIME type. However, modifying the UI library has broader implications.

**Decision**: Extend `accept` to include both MIME types AND extensions: `image/png,image/jpeg,image/gif,image/webp,application/pdf,.md,.txt,.json,.csv,.ts,.js,.tsx,.jsx,.log,.yaml,.yml,.toml,.env,.html,.css,.xml`. The native `<input>` will filter by extension, and `matchesAccept` already has `""` fallback: if accept is non-empty and file type doesn't match, it's rejected. Text files with empty MIME will be caught by this.

**Correction**: After reviewing `matchesAccept` at line 394-414, if `accept` is set and a file has `f.type === ""`, it will NOT match any pattern and will be rejected. This means we need to also modify `matchesAccept` to allow files with empty MIME when accept includes extension patterns. 

**Final decision**: Add extension-based matching in `matchesAccept`: if `accept` includes patterns starting with `.` (extensions), match against `file.name` extension instead of `file.type`. This is a targeted change to `prompt-input.tsx`.

### 3. Place `AttachButton` before `PromptToolbar` in both views

**Rationale**: User expectation — the attach button is a document action (like in email clients), and should appear before the agent/model/variant selectors which modify how the message is processed. In `chat-view.tsx`, this order already exists (line 1425: `<AttachButton>` then line 1426: `<PromptToolbar>`). In `new-chat.tsx`, the `AttachButton` will be added before `PromptToolbar` in the same `PromptInputTools` group.

### 4. Increase maxFileSize to 25MB

**Rationale**: 10MB is constraining for text/code files in large repositories (e.g., a big JSON or CSV export, or a minified JS bundle for debugging). 25MB provides headroom while staying reasonable for local filesystem and blob URL memory usage. This is a simple constant change in both `chat-view.tsx` and `new-chat.tsx`.

**Alternative considered**: Making maxFileSize configurable. Rejected as premature — 25MB is a reasonable default, and configuration can be added later if needed.

### 5. Extract `AttachButton` to a shared module

**Rationale**: Both `chat-view.tsx` and `new-chat.tsx` need the same button. Extract to a new file `apps/desktop/src/renderer/components/chat/attach-button.tsx` (or co-locate in an existing shared file).

**Alternative considered**: Co-locating in `prompt-toolbar.tsx`. Rejected — the attach button is a separate concern from the toolbar (agent/model selectors).

## Risks / Trade-offs

- **[Risk] Extension-based file filtering is less reliable than MIME**: `.ts` and `.tsx` files on some OS setups may report `video/mp2t` MIME, causing false rejection. → **Mitigation**: If accept includes both MIME and extension patterns, extension patterns should be checked via filename, not MIME. This requires the `matchesAccept` change described in Decision 2.
- **[Risk] Text files with empty MIME**: Many text editors and OS file dialogs report `""` as MIME for `.txt`, `.md`, etc. → **Mitigation**: `matchesAccept` extension fallback handles this.
- **[Trade-off] Broader accept types may clutter the file picker**: Adding many extensions to the OS file dialog accept filter changes what files are shown by default. → **Acceptable**: This is the standard UX for tools that accept multiple file types (like GitHub, ChatGPT, etc.).
- **[Risk] 25MB files increase memory pressure**: Large files loaded as blob URLs stay in memory. → **Mitigation**: File attachments are transient (cleared after send), and 25MB is a modest limit for modern Electron apps with multi-GB heaps.

## Open Questions

- Should we add a model capability warning for text files similar to the existing image/PDF warning in `PromptAttachmentPreview`? → **Decision deferred**: Not required for initial implementation; most models accept text inputs.
