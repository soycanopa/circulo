## Why

Circulo users have no dedicated place to see what skills are available, which ones are installed, or how to discover new ones. Currently, skills are only surfaced through the `/skills` slash-command picker inside a chat, which shows a flat list from the OpenCode server with no distinction between global and project-local skills, and no way to browse or install skills from the skills.sh ecosystem. As the agent skills ecosystem grows, users need a proper settings page to manage their skill inventory.

## What Changes

- Add a **Skills** tab to the Settings sidebar that shows all installed skills (global + per-project)
- Group skills by origin: globally-installed (`~/.config/opencode/skills/`), project-local, and Cursor-migrated
- Show skill metadata: name, description, location path, and which projects reference each skill
- Add a **Discover** sub-section that integrates with `skills.sh` to browse trending/popular skills
- Allow installing skills from skills.sh via `npx skills add <owner/repo>`, surfaced through the UI, with a target selector to install globally or into a specific registered project
- Add an IPC channel so the main process can scan skill directories and report installed skills to the renderer

## Capabilities

### New Capabilities

- `skill-inventory`: Enumerate installed skills from all configured directories (global, project-local, Cursor-migrated), with metadata and project associations
- `skill-discovery`: Browse and search skills from skills.sh, view details (description, installs, author), and install via `npx skills add` — with a target selector (global or per-project)
- `skill-settings-page`: A dedicated Skills tab in the Settings UI that hosts both inventory and discovery views

### Modified Capabilities

<!-- No existing specs are modified -->

## Impact

- **New files**: `skill-settings.tsx` (settings page), `use-installed-skills.ts`, `use-skill-discovery.ts`, `use-skill-install.ts` (hooks), main process `skills.ts` for directory scanning and installation
- **Modified files**: `router.tsx` (new route), `settings-page.tsx` (new tab), `api.d.ts` (new IPC methods)
- **New dependency**: `skills.sh` API (read-only, for browsing) + `npx skills` CLI (for installation, already available via Node.js)
- **Electron main process**: New IPC handlers to scan `~/.config/opencode/skills/` and project-local directories, plus shell execution of `npx skills add`
- **No breaking changes** to existing settings, IPC, or chat functionality
