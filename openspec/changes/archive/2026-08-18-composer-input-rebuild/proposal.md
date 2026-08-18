# Proposal: Composer input rebuild

## Why

The composer uses a key-buffer on `AppShell` with a bolted-on IME path. It does not behave like a native text field on macOS (no reliable focus, caret, or IME). This blocks manual testing of live streaming and core FLOWS §4–6.

## What

Replace the key-buffer with a dedicated `ComposerInput` GPUI entity (Waku-inspired architecture, no copied code) and a `Composer` container for toolbar + send. Add per-session draft restore, read-only while generating, draft retention on send error, and PATCH project on picker select.

## Impact

- `circulo-app`: new `composer/` module; `AppShell` slimmed down
- `circulo-i18n`: agent selector copy
- Delta spec: `composer-stream`

## Non-goals

Autocomplete, attachments, cancel/stop, undo history, gpui-component dependency.
