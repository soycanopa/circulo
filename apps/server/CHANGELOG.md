# @circulo/server

## 0.12.0

### Minor Changes

- Minor bump: sidebar redesign with archive and hover buttons, chat persistence, keyboard shortcuts (Cmd+N, Opt+Cmd+N), slash commands and skills dialog, frosted glass effect, interactive server indicator, new wordmark, project selector redesign, and settings keyboard shortcuts page.

## 0.11.2

### Patch Changes

- Fix cross-referenced issues from palot: platform-aware local server name, hardcoded color fixes, hide opaque toggle on non-macOS, cancel queued chat messages, read-only branch display for remote projects, unread dot badge on sidebar sessions, and configurable opencode binary path

## 0.11.1

### Patch Changes

- Bump dependencies: 14 Dependabot PRs including major updates (electron 42, recharts 3, shadcn 4, zod 4, shiki 4, lucide-react 1, react-day-picker 10) and minor updates (hono, dev-tools, changelog-github, CI actions). Fixed breaking changes in react-day-picker (classNames table→month_grid) and recharts (Tooltip/Legend v3 types).

## 0.4.0

### Patch Changes

- [`2a9d3de`](https://github.com/soycanopa/circulo/commit/2a9d3de3b529a6aa73b8ae574fb2a6f2084d73f9) Thanks [@ItsWendell](https://github.com/ItsWendell)! - ### Breaking: Require OpenCode >= 1.2.0

  Circulo now requires OpenCode 1.2.0 or higher. Older versions will be blocked during the environment check.

  ### New Features

  - **Incremental streaming (message.part.delta)**: handle the new `message.part.delta` SSE event for incremental text/reasoning updates, replacing full part object replacements with efficient string delta appending
  - **Remote server support**: connect to remote OpenCode servers with authentication, mDNS discovery, and onboarding integration
  - **Update banner redesign**: floating toast card instead of full-width bar

  ### SDK Migration

  - Upgrade `@opencode-ai/sdk` from 1.1.x to 1.2.x across all packages
  - Migrate all SDK type imports from v1 to v2, gaining proper typed discriminated unions for all events
  - `Permission` type replaced by `PermissionRequest` (field `title` becomes `permission`)
  - `permission.updated` event replaced by `permission.asked`
  - Remove unnecessary type casts throughout the event processing pipeline

  ### Fixes

  - Fix CI release workflow to run builds in a single workflow run
  - Suppress hover background on macOS sidebar server indicator

## 0.2.0

### Patch Changes

- [`d2d6f2b`](https://github.com/soycanopa/circulo/commit/d2d6f2b3013ad0fa3bb9ac08ad9b8ff91517ffc5) Thanks [@ItsWendell](https://github.com/ItsWendell)! - ### New Features

  - Add provider management with icons, catalog, and onboarding integration
  - Add git worktree backend with lifecycle management and UI
  - Add automations subsystem with database, scheduler, and IPC (hidden for now)

  ### Improvements

  - Improve Electron main process reliability
  - Improve command palette animation and chat UX
  - Increase spacing between expanded tool call items in chat turns

  ### Fixes

  - Upgrade hono to 4.11.9 to resolve security alerts
  - Resolve type errors in chat-input component
