## 1. Shared AttachButton component

- [x] 1.1 Extract `AttachButton` from `chat-view.tsx` into `apps/desktop/src/renderer/components/chat/attach-button.tsx`
- [x] 1.2 Import and reuse shared `AttachButton` in `chat-view.tsx` (remove local definition)
- [x] 1.3 Import and render shared `AttachButton` in `new-chat.tsx` inside `<PromptInputTools>` before `<PromptToolbar>`

## 2. File type filtering

- [x] 2.1 Add extension-based matching in `packages/ui/src/components/ai-elements/prompt-input.tsx` `matchesAccept`: if `accept` includes patterns starting with `.`, match against `file.name` extension in addition to `file.type`
- [x] 2.2 Define the extended `accept` string constant with image MIME types, PDF MIME, and text/code extensions (`.md`, `.txt`, `.json`, `.csv`, `.ts`, `.js`, `.tsx`, `.jsx`, `.log`, `.yaml`, `.yml`, `.toml`, `.env`, `.html`, `.css`, `.xml`, `.py`, `.rb`, `.go`, `.rs`, `.java`, `.c`, `.h`, `.cpp`, `.sql`)

## 3. Update chat-view

- [x] 3.1 Replace hardcoded `accept` string in `<PromptInput>` with shared constant from step 2.2
- [x] 3.2 Increase `maxFileSize` from `10 * 1024 * 1024` to `25 * 1024 * 1024`
- [x] 3.3 Verify existing `AttachButton` placement is before `PromptToolbar` in `<PromptInputTools>` (already correct — just confirm)

## 4. Update new-chat

- [x] 4.1 Replace hardcoded `accept` string in `<PromptInput>` with shared constant from step 2.2
- [x] 4.2 Increase `maxFileSize` from `10 * 1024 * 1024` to `25 * 1024 * 1024`
- [x] 4.3 Add `AttachButton` inside `<PromptInputTools>` before `<PromptToolbar>` (currently only toolbar is rendered, no `AttachButton`)

## 5. Verification

- [x] 5.1 Run `bun run check-types` from root to verify no type errors
- [x] 5.2 Run `bun run lint` from root to verify no lint violations
