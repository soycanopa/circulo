## Why

Circulo still has no window. The next product surface is a native GPUI shell: hidden title bar, traffic lights in the sidebar, a collapse rail, and English UI strings loaded from locales.

## What Changes

- Open a macOS GPUI window with a transparent/custom title bar.
- Place traffic lights in the sidebar top bar, aligned with the hide/show control.
- Collapsed sidebar is a min rail; traffic lights stay there.
- Dark shell layout: sidebar + header/messages/composer placeholders.
- Load all visible copy from `circulo-i18n` locale `en`.
- Does **not** wire the daemon, session list data, or rich chat yet.

## Capabilities

### New Capabilities

- `app-shell`: Native window chrome and AppShell layout (sidebar rail, locales).

### Modified Capabilities

- (none)

## Non-goals

- No live sessions, search, settings panel, or composer send.
- No Heroicons pipeline yet (text/label control for hide).
- No daemon spawn from the app (still open).
- No HTTPS.

## Impact

- `circulo-app` depends on `gpui` and `circulo-i18n`.
- `circulo-i18n` ships `en` catalog.

## Open questions (not resolved here)

- App spawn/reuse of the daemon.
- Exact hex palette beyond a small dark set chosen for this shell.
