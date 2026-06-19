## 1. Main Process — IPC Handlers & Skill Scanning

- [x] 1.1 Create `apps/desktop/src/main/skills.ts` with `scanSkills()` function that scans all skill directories (global, project-local, Cursor) and returns consolidated skill list with metadata
- [x] 1.2 Implement `extractSkillMetadata()` helper that reads a skill's SKILL.md and extracts name/description from YAML frontmatter or first line fallback
- [x] 1.3 Register `skills:list` IPC handler in `apps/desktop/src/main/ipc-handlers.ts` that calls `scanSkills()` and returns results
- [x] 1.4 Implement `installSkill()` function that takes `{ ownerRepo, target? }` — for global installs, runs `npx skills add <owner/repo>` with 30s timeout; for per-project installs, clones the repo into `<target>/.agents/skills/<name>/`
- [x] 1.5 Register `skills:install` IPC handler that calls `installSkill()` and returns `{ success, error? }` result
- [x] 1.6 Implement `getRegisteredProjects()` helper that reads registered projects from the session store for the target selector in UI
- [x] 1.7 Add installation queue to serialize concurrent install calls

## 2. Preload — API Types & Bridge

- [x] 2.1 Add `InstalledSkill` interface, `SkillOrigin` type, `InstallResult` interface, and `DiscoveredSkill` interface to `apps/desktop/src/preload/api.d.ts`
- [x] 2.2 Add `listSkills: () => Promise<InstalledSkill[]>` and `installSkill: (params: { ownerRepo: string; target?: string }) => Promise<InstallResult>` to `CirculoAPI` interface in `apps/desktop/src/preload/api.d.ts`
- [x] 2.3 Wire `skills:list` and `skills:install` IPC channels in `apps/desktop/src/preload/index.ts` via `ipcRenderer.invoke`

## 3. Renderer — Hooks

- [x] 3.1 Create `apps/desktop/src/renderer/hooks/use-installed-skills.ts` hook that calls `window.circulo.listSkills()` and returns grouped skills by origin
- [x] 3.2 Create `apps/desktop/src/renderer/hooks/use-skill-discovery.ts` hook that fetches trending skills from skills.sh API with search support
- [x] 3.3 Create `apps/desktop/src/renderer/hooks/use-skill-install.ts` hook that calls `window.circulo.installSkill()` and manages install state (idle, installing, success, error)

## 4. Renderer — Skills Settings Page Components

- [x] 4.1 Create `apps/desktop/src/renderer/components/settings/skill-settings.tsx` — main Skills settings page component with "Installed" / "Discover" sub-tabs
- [x] 4.2 Build the **Installed** sub-view: grouped skill list (Global, Project, Cursor) using `SettingsSection` + `SettingsRow`, with skill count badges, alphabetical sort, and "No skills installed" empty state
- [x] 4.3 Build the **Discover** sub-view: search input, trending skills grid from skills.sh, each skill card showing name, description, installs, author
- [x] 4.4 Build skill detail panel component: shows full skill details when clicked (for installed: name/description/location/origin; for discover: name/description/installs/author/repo with Install button and target selector dropdown)
- [x] 4.5 Build target selector dropdown in Discover detail panel: "Global" + list of registered projects, defaults to "Global"
- [x] 4.6 Add Refresh button to Installed view that triggers `window.circulo.listSkills()` rescan
- [x] 4.7 Wire Install button in Discover view to call `window.circulo.installSkill({ ownerRepo, target })` with loading/success/error states

## 5. Router & Navigation Integration

- [x] 5.1 Import `SkillSettings` component in `apps/desktop/src/renderer/router.tsx` and create `settingsSkillsRoute` child route with path `"skills"`
- [x] 5.2 Add `settingsSkillsRoute` to the route tree children array in `router.tsx`
- [x] 5.3 Add `"skills"` to the `SettingsTab` type union in `apps/desktop/src/renderer/components/settings/settings-page.tsx`
- [x] 5.4 Add `{ id: "skills", label: "Skills", icon: BookOpenIcon }` to the `tabs` array in `settings-page.tsx`

## 6. Verification

- [x] 6.1 Verify the Skills tab appears in Settings sidebar and navigation works
- [x] 6.2 Verify installed skills list shows skills from `~/.config/opencode/skills/` grouped by origin
- [x] 6.3 Verify Discover tab loads trending skills from skills.sh
- [x] 6.4 Verify skill installation (both global and per-project) works end-to-end
- [x] 6.5 Verify target selector dropdown shows registered projects
- [x] 6.6 Run `bun run lint` and `bun run check-types` to verify no regressions
