## Context

Circulo is an Electron desktop app with a React renderer. Skills are currently only surfaced through a slash-command picker (`/skills`) that queries the OpenCode server via `client.app.skills()`. There is no settings page for browsing or managing skills, and no integration with the skills.sh ecosystem.

Current skill directories:
- `~/.config/opencode/skills/` — globally installed skills (one subdirectory per skill)
- `~/.cursor/skills/` — migrated from Cursor during onboarding
- Project-local: skills in the project's `.agents/skills/` or `.opencode/skills/` directories

The OpenCode server already has the logic to discover skills from these directories, but the renderer has no direct access to the filesystem — it must go through the Electron main process via IPC.

## Goals / Non-Goals

**Goals:**
- A Skills tab in Settings that shows all installed skills grouped by origin (global, project-local, Cursor-migrated)
- A Discovery sub-view that lets users browse skills.sh, search, and install skills via `npx skills add`
- Main-process IPC handlers that scan the filesystem for installed skills and execute `npx` for installation
- Per-project skill associations visible in the UI

**Non-Goals:**
- Creating, editing, or deleting skills from the UI (out of scope — use filesystem or CLI)
- Managing skill permissions or agent configuration (out of scope)
- Skills.sh publisher API (read-only consumption only)
- Replacing the existing `/skills` slash-command picker

## Decisions

### 1. IPC: Main process scans directories, renderer consumes results

**Decision**: New IPC channels in the main process (`skills:list`, `skills:install`) that the renderer calls via `window.circulo`.

**Rationale**: The renderer is sandboxed and cannot access the filesystem. The main process already has patterns for this (settings store, onboarding migration). Using IPC keeps the skill data flow consistent with how settings, servers, and providers work.

**Alternatives considered**:
- Extend the OpenCode server API — rejected because the server may not be running, and skill directory management is a client concern
- Use `fs` in preload — rejected because it's against Electron best practices and our preload only exposes structured APIs

### 2. Skill inventory: Scan proactively, not per-request

**Decision**: The main process scans known skill directories on startup and whenever the Skills settings page is opened. Results are cached until a rescan is triggered.

**Rationale**: Skill directories change infrequently (only when installing/removing). Scanning on every render would be wasteful but stale data is confusing. A manual "Refresh" button plus automatic scan-on-open balances freshness with performance.

### 3. Skills.sh integration: Use the skills.sh API + `npx skills add` CLI

**Decision**: Browse/discovery uses the skills.sh REST API (read-only). Installation shell-executes `npx skills add <owner/repo>` in the main process.

**Rationale**: skills.sh provides an API for listing/searching skills. For installation, there's no programmatic API — the canonical way is `npx skills add`. The main process can spawn this as a child process and report progress/errors via IPC events.

**Alternatives considered**:
- Direct filesystem manipulation (cloning repos) — rejected because it bypasses skills.sh conventions and would break with future changes
- Webview for skills.sh browsing — rejected because we explicitly avoid external URLs in Electron windows per project conventions

### 5. Install target: Global vs per-project selector

**Decision**: The Discover detail panel includes a dropdown to select the install target: "Global" (default, installs to `~/.config/opencode/skills/`) or any registered project (installs to `<project>/.agents/skills/<name>/`). The IPC `skills:install` channel accepts `{ ownerRepo, target? }` — if `target` is omitted, installs globally.

**Rationale**: `npx skills add` installs globally by default and has no `--target` flag. For per-project installs, the main process clones the skill repository directly into `<target>/.agents/skills/<name>/` and extracts metadata. This gives users full control over where skills are available — global for all projects, or scoped to a specific project's agent context.

**Alternatives considered**:
- Only global install — rejected because users need project-specific skills (e.g., `react-best-practices` in one project but not another)
- `npx skills add --target <path>` — rejected because the CLI doesn't support this flag

### 4. UI layout: Two tabs within the Skills settings page

**Decision**: The Skills settings page has two sub-tabs: "Installed" (inventory) and "Discover" (skills.sh browser). Both live under the single `/settings/skills` route.

**Rationale**: Keeps routing simple (one route, internal tab state). Follows existing patterns — the Provider settings page already has a similar connect/disconnect list layout within a single route.

## Risks / Trade-offs

- **[Risk] `npx skills add` may fail or hang** — Mitigation: Run with a 30s timeout in the main process, report errors via IPC, show loading state with cancel option in UI
- **[Risk] skills.sh API may change or be unavailable** — Mitigation: Graceful degradation — show a "Could not load skills.sh" message with a retry button. Cached results from last successful fetch
- **[Risk] Scanning many project directories could be slow** — Mitigation: Only scan registered project directories (from thread list), not the entire filesystem. Lazy-load per-project details on demand
- **[Risk] Parallel `npx skills add` invocations could conflict** — Mitigation: Queue installation requests, process sequentially with UI feedback

## Open Questions

- Should the Skills page show skills that are available but not installed yet (e.g., discovered via skills.sh)?
- Should the UI allow uninstalling skills, or is that out of scope?
