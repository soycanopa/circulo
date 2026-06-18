# RTK Integration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an RTK toggle to Circulo's General Settings that transparently manages the OpenCode RTK plugin -- checking for the `rtk` binary, installing/removing the plugin, and restarting the OpenCode server.

**Architecture:** New `main/rtk/` directory with an embedded plugin source and a service module. The service is wired through IPC handlers, the preload bridge, a backend service, and a Jotai atom to a Switch toggle in the General Settings page.

**Tech Stack:** Node.js (main process), TypeScript (shared types), React 19 + Jotai (renderer), Electron IPC

## Global Constraints

- Must work inside Electron only (browser-mode dev has no OpenCode plugins folder)
- Plugin file goes to `~/.config/opencode/plugins/rtk.ts`
- Server restart is required after plugin install/uninstall
- RTK binary detection uses `which rtk` spawn
- Installation help opens the system terminal (macOS: `open -a Terminal`, Linux: xterm fallback)
- Follow existing patterns: `withLogging()` for IPC handlers, `atomWithStorage` for persisted atoms, thin backend service functions

---

## File Structure

| File | Action | Purpose |
|---|---|---|
| `apps/desktop/src/main/rtk/plugin-source.ts` | Create | 22-line OpenCode plugin as a string constant |
| `apps/desktop/src/main/rtk/rtk-service.ts` | Create | Main process: check binary, manage plugin file, open terminal |
| `apps/desktop/src/preload/api.d.ts` | Modify | Add `rtkEnabled` to AppSettings, add `RtkCheckResult` type, add rtk API to CirculoAPI |
| `apps/desktop/src/main/settings-store.ts` | Modify | Add `rtkEnabled: false` default, add `getRtkEnabled()` |
| `apps/desktop/src/main/ipc-handlers.ts` | Modify | Add `rtk:check`, `rtk:enable`, `rtk:disable`, `rtk:install` handlers |
| `apps/desktop/src/preload/index.ts` | Modify | Add `rtk` bridge object to `window.circulo` |
| `apps/desktop/src/renderer/atoms/preferences.ts` | Modify | Add `rtkEnabledAtom` |
| `apps/desktop/src/renderer/services/backend.ts` | Modify | Add `checkRtk`, `enableRtk`, `disableRtk`, `installRtk` |
| `apps/desktop/src/renderer/components/settings/general-settings.tsx` | Modify | Add `RtkRow` toggle component in new "Token Optimization" section |

---

### Task 1: Embedded RTK Plugin Source

**Files:**
- Create: `apps/desktop/src/main/rtk/plugin-source.ts`

**Interfaces:**
- Produces: `export const RTK_PLUGIN_SOURCE: string` -- the full 22-line TypeScript plugin as a string constant

- [ ] **Step 1: Create the embedded plugin source file**

```ts
// File: apps/desktop/src/main/rtk/plugin-source.ts
// Embedded RTK OpenCode plugin source, from https://github.com/rtk-ai/rtk
// This is written to ~/.config/opencode/plugins/rtk.ts when RTK is enabled.

export const RTK_PLUGIN_SOURCE = `import type { Plugin } from "@opencode-ai/plugin"

// RTK OpenCode plugin — rewrites commands to use rtk for token savings.
// Requires: rtk >= 0.23.0 in PATH.

export const RtkOpenCodePlugin: Plugin = async ({ $ }) => {
  try {
    await $\`which rtk\`.quiet()
  } catch {
    console.warn("[rtk] rtk binary not found in PATH — plugin disabled")
    return {}
  }

  return {
    "tool.execute.before": async (input, output) => {
      const tool = String(input?.tool ?? "").toLowerCase()
      if (tool !== "bash" && tool !== "shell") return
      const args = output?.args
      if (!args || typeof args !== "object") return

      const command = (args as Record<string, unknown>).command
      if (typeof command !== "string" || !command) return

      try {
        const result = await $\`rtk rewrite \${command}\`.quiet().nothrow()
        const rewritten = String(result.stdout).trim()
        if (rewritten && rewritten !== command) {
          ;(args as Record<string, unknown>).command = rewritten
        }
      } catch {
        // rtk rewrite failed — pass through unchanged
      }
    },
  }
}
`
```

- [ ] **Step 2: Verify the file compiles**

```bash
cd apps/desktop && bun run check-types
```

Expected: passes (only a string constant, no dependencies)

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/src/main/rtk/plugin-source.ts
git commit -m "feat(rtk): embed RTK OpenCode plugin source"
```

---

### Task 2: RTK Service (Main Process)

**Files:**
- Create: `apps/desktop/src/main/rtk/rtk-service.ts`

**Interfaces:**
- Consumes: `RTK_PLUGIN_SOURCE` from `./plugin-source`
- Produces:
  - `isRtkInstalled(): Promise<boolean>`
  - `getRtkVersion(): Promise<string | null>`
  - `installPlugin(): Promise<boolean>`
  - `uninstallPlugin(): Promise<void>`
  - `isPluginActive(): boolean`
  - `openTerminalForInstall(): void`

- [ ] **Step 1: Create the rtk-service.ts file**

```ts
// File: apps/desktop/src/main/rtk/rtk-service.ts
import { execSync, spawn } from "node:child_process"
import fs from "node:fs"
import path from "node:path"
import { homedir } from "node:os"
import { createLogger } from "../logger"
import { RTK_PLUGIN_SOURCE } from "./plugin-source"

const log = createLogger("rtk-service")

// ============================================================
// Paths
// ============================================================

const OPENCODE_PLUGINS_DIR = path.join(homedir(), ".config", "opencode", "plugins")
const PLUGIN_FILE = path.join(OPENCODE_PLUGINS_DIR, "rtk.ts")

// ============================================================
// Binary detection
// ============================================================

/** Check if the `rtk` binary is installed and on PATH. */
export function isRtkInstalled(): boolean {
	try {
		execSync("which rtk", { stdio: "ignore" })
		return true
	} catch {
		return false
	}
}

/** Get the installed version of `rtk`, or null if not installed. */
export function getRtkVersion(): string | null {
	try {
		const output = execSync("rtk --version", { encoding: "utf-8", stdio: ["ignore", "pipe", "ignore"] })
		const match = output.match(/(\d+\.\d+\.\d+)/)
		return match ? match[1] : output.trim()
	} catch {
		return null
	}
}

// ============================================================
// Plugin management
// ============================================================

/** Write the RTK plugin to the OpenCode plugins directory. Returns true on success. */
export function installPlugin(): boolean {
	try {
		if (!fs.existsSync(OPENCODE_PLUGINS_DIR)) {
			fs.mkdirSync(OPENCODE_PLUGINS_DIR, { recursive: true })
		}
		fs.writeFileSync(PLUGIN_FILE, RTK_PLUGIN_SOURCE, "utf-8")
		log.info("RTK plugin installed", { path: PLUGIN_FILE })
		return true
	} catch (err) {
		log.error("Failed to install RTK plugin", err)
		return false
	}
}

/** Remove the RTK plugin from the OpenCode plugins directory. */
export function uninstallPlugin(): void {
	try {
		if (fs.existsSync(PLUGIN_FILE)) {
			fs.unlinkSync(PLUGIN_FILE)
			log.info("RTK plugin uninstalled", { path: PLUGIN_FILE })
		}
	} catch (err) {
		log.error("Failed to uninstall RTK plugin", err)
	}
}

/** Check if the RTK plugin file exists. */
export function isPluginActive(): boolean {
	return fs.existsSync(PLUGIN_FILE)
}

// ============================================================
// Terminal install helper
// ============================================================

/** Open the system terminal with the RTK install command pre-loaded. */
export function openTerminalForInstall(): void {
	const installCmd =
		'curl -fsSL https://raw.githubusercontent.com/rtk-ai/rtk/refs/heads/master/install.sh | sh'

	if (process.platform === "darwin") {
		const script = `tell application "Terminal" to do script "clear; echo 'Installing RTK...'; ${installCmd}"`
		spawn("osascript", ["-e", script], { detached: true, stdio: "ignore" }).unref()
	} else if (process.platform === "linux") {
		// Try common terminals in order
		const terminals = ["gnome-terminal", "konsole", "xterm"]
		for (const term of terminals) {
			try {
				execSync(`which ${term}`, { stdio: "ignore" })
				if (term === "gnome-terminal") {
					spawn(term, ["--", "bash", "-c", `echo 'Installing RTK...'; ${installCmd}; exec bash`], {
						detached: true,
						stdio: "ignore",
					}).unref()
				} else if (term === "konsole") {
					spawn(term, ["-e", "bash", "-c", `echo 'Installing RTK...'; ${installCmd}; exec bash`], {
						detached: true,
						stdio: "ignore",
					}).unref()
				} else {
					spawn(term, ["-e", `echo 'Installing RTK...'; ${installCmd}; exec bash`], {
						detached: true,
						stdio: "ignore",
					}).unref()
				}
				return
			} catch {
				// Try next terminal
			}
		}
		log.warn("No supported terminal found on Linux")
	}
	// Windows: open cmd.exe
	if (process.platform === "win32") {
		spawn("cmd.exe", ["/c", "start", "cmd.exe", "/k", installCmd], {
			detached: true,
			stdio: "ignore",
		}).unref()
	}
}
```

- [ ] **Step 2: Verify the file compiles**

```bash
cd apps/desktop && bun run check-types
```

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/src/main/rtk/rtk-service.ts
git commit -m "feat(rtk): add RTK service for binary detection and plugin management"
```

---

### Task 3: Update Types and Settings Store

**Files:**
- Modify: `apps/desktop/src/preload/api.d.ts:209-217` (AppSettings), add new types
- Modify: `apps/desktop/src/main/settings-store.ts:16-26` (DEFAULT_SETTINGS), `96-111` (add getter)

**Interfaces:**
- Produces: `RtkCheckResult` interface, `rtkEnabled` field on `AppSettings`, `getRtkEnabled()` in settings-store

- [ ] **Step 1: Add RtkCheckResult type and rtkEnabled to AppSettings in api.d.ts**

Insert after line 217 (closing brace of AppSettings):

```ts
// In api.d.ts, after line 217:
export interface RtkCheckResult {
  installed: boolean
  version: string | null
  pluginActive: boolean
  enabled: boolean
}
```

Modify `AppSettings` interface (lines 209-217) to add `rtkEnabled`:

```ts
export interface AppSettings {
	notifications: NotificationSettings
	opaqueWindows: boolean
	servers: ServerSettings
	opencodeBinary?: string
	/** Whether RTK token optimization is enabled. When true, the OpenCode plugin is installed. */
	rtkEnabled?: boolean
}
```

- [ ] **Step 2: Add CirculoAPI rtk methods to api.d.ts**

After line 545 (`onSettingsChanged`), add:

```ts
	// RTK integration
	rtk: {
		/** Check if RTK is installed and the plugin is active. */
		check: () => Promise<RtkCheckResult>
		/** Enable RTK: install plugin + restart server. */
		enable: () => Promise<{ success: boolean; error?: string }>
		/** Disable RTK: remove plugin + restart server. */
		disable: () => Promise<{ success: boolean }>
		/** Open terminal with RTK install command. */
		install: () => Promise<void>
	}
```

- [ ] **Step 3: Update DEFAULT_SETTINGS in settings-store.ts line 16-26**

Add `rtkEnabled: false`:

```ts
const DEFAULT_SETTINGS: AppSettings = {
	notifications: {
		completionMode: "unfocused",
		permissions: true,
		questions: true,
		errors: true,
		dockBadge: true,
	},
	opaqueWindows: false,
	rtkEnabled: false,
	servers: DEFAULT_SERVER_SETTINGS,
}
```

- [ ] **Step 4: Add getRtkEnabled() getter after getOpencodeBinary() line 111**

```ts
/** Get the RTK enabled state. */
export function getRtkEnabled(): boolean {
	return settings.rtkEnabled === true
}
```

- [ ] **Step 5: Verify types compile**

```bash
cd apps/desktop && bun run check-types
```

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/src/preload/api.d.ts apps/desktop/src/main/settings-store.ts
git commit -m "feat(rtk): add RTK types and settings store support"
```

---

### Task 4: Add IPC Handlers

**Files:**
- Modify: `apps/desktop/src/main/ipc-handlers.ts`

**Interfaces:**
- Consumes: `isRtkInstalled`, `getRtkVersion`, `installPlugin`, `uninstallPlugin`, `isPluginActive`, `openTerminalForInstall` from `./rtk/rtk-service`
- Consumes: `getRtkEnabled`, `updateSettings` from `./settings-store`
- Consumes: `restartServer` from `./opencode-manager`

- [ ] **Step 1: Add imports to ipc-handlers.ts (after line 52)**

After the `getSettings`, `onSettingsChanged`, `updateSettings` import line:

```ts
import { getRtkEnabled } from "./settings-store"
import { getRtkVersion, installPlugin, isPluginActive, isRtkInstalled, openTerminalForInstall, uninstallPlugin } from "./rtk/rtk-service"
```

- [ ] **Step 2: Add IPC handlers in registerIpcHandlers()**

Insert after line 637 (end of `onSettingsChanged` block), before the closing brace of `registerIpcHandlers` at line 638:

```ts
	// --- RTK integration ---

	ipcMain.handle("rtk:check", () => {
		const installed = isRtkInstalled()
		const version = getRtkVersion()
		const pluginActive = isPluginActive()
		const enabled = getRtkEnabled()
		return { installed, version, pluginActive, enabled }
	})

	ipcMain.handle(
		"rtk:enable",
		withLogging("rtk:enable", async () => {
			if (!isRtkInstalled()) {
				return { success: false, error: "RTK is not installed" }
			}
			const ok = installPlugin()
			if (!ok) {
				return { success: false, error: "Failed to write RTK plugin file" }
			}
			updateSettings({ rtkEnabled: true })
			await restartServer()
			return { success: true }
		}),
	)

	ipcMain.handle(
		"rtk:disable",
		withLogging("rtk:disable", async () => {
			uninstallPlugin()
			updateSettings({ rtkEnabled: false })
			await restartServer()
			return { success: true }
		}),
	)

	ipcMain.handle("rtk:install", () => {
		openTerminalForInstall()
	})
```

- [ ] **Step 3: Verify types compile**

```bash
cd apps/desktop && bun run check-types
```

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/src/main/ipc-handlers.ts
git commit -m "feat(rtk): add IPC handlers for RTK integration"
```

---

### Task 5: Preload Bridge

**Files:**
- Modify: `apps/desktop/src/preload/index.ts`

- [ ] **Step 1: Add rtk bridge object before automations section (before line 249)**

Insert the rtk bridge object after the `onSettingsChanged` block:

```ts
	// --- RTK integration ---

	rtk: {
		/** Check if RTK is installed and the plugin is active. */
		check: () => ipcRenderer.invoke("rtk:check"),
		/** Enable RTK: install plugin + restart server. */
		enable: () => ipcRenderer.invoke("rtk:enable"),
		/** Disable RTK: remove plugin + restart server. */
		disable: () => ipcRenderer.invoke("rtk:disable"),
		/** Open terminal with RTK install command. */
		install: () => ipcRenderer.invoke("rtk:install"),
	},
```

Place it after the `onSettingsChanged` closing brace on line 247 (before `// --- Automations ---` on line 249).

- [ ] **Step 2: Verify types compile**

```bash
cd apps/desktop && bun run check-types
```

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/src/preload/index.ts
git commit -m "feat(rtk): add RTK preload bridge methods"
```

---

### Task 6: Renderer Atom and Backend Service

**Files:**
- Modify: `apps/desktop/src/renderer/atoms/preferences.ts`
- Modify: `apps/desktop/src/renderer/services/backend.ts`

**Interfaces:**
- Produces: `rtkEnabledAtom` (Jotai atom, persisted), `checkRtk()`, `enableRtk()`, `disableRtk()`, `installRtk()` in backend

- [ ] **Step 1: Add rtkEnabledAtom to preferences.ts (after line 112)**

After `favoriteModelsAtom`:

```ts
/**
 * Whether RTK token optimization is enabled.
 * When true, Circulo manages the OpenCode RTK plugin and restarts the server.
 */
export const rtkEnabledAtom = atomWithStorage<boolean>("circulo:rtkEnabled", false)
```

- [ ] **Step 2: Add import for RtkCheckResult in backend.ts**

After the imports (line 12-27), add `RtkCheckResult` to the import list:

```ts
import type {
	Automation,
	AutomationRun,
	CreateAutomationInput,
	GitApplyResult,
	GitBranchInfo,
	GitCheckoutResult,
	GitCommitResult,
	GitDiffStat,
	GitPushResult,
	GitStashResult,
	GitStatusInfo,
	ModelState,
	OpenInTargetsResult,
	RtkCheckResult,
	UpdateAutomationInput,
} from "../../preload/api"
```

- [ ] **Step 3: Add RTK backend service functions at end of backend.ts (before end)**

```ts
// ============================================================
// RTK integration — Electron-only
// ============================================================

/** Check if RTK is installed, its version, and whether the plugin is active. */
export async function checkRtk(): Promise<RtkCheckResult> {
	if (isElectron) {
		return window.circulo.rtk.check()
	}
	return { installed: false, version: null, pluginActive: false, enabled: false }
}

/** Enable RTK: install the OpenCode plugin and restart the server. */
export async function enableRtk(): Promise<{ success: boolean; error?: string }> {
	if (isElectron) {
		return window.circulo.rtk.enable()
	}
	return { success: false, error: "RTK is only available in Electron mode" }
}

/** Disable RTK: remove the OpenCode plugin and restart the server. */
export async function disableRtk(): Promise<{ success: boolean }> {
	if (isElectron) {
		return window.circulo.rtk.disable()
	}
	return { success: false }
}

/** Open terminal to install RTK. */
export async function installRtk(): Promise<void> {
	if (isElectron) {
		await window.circulo.rtk.install()
	}
}
```

- [ ] **Step 4: Verify types compile**

```bash
cd apps/desktop && bun run check-types
```

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src/renderer/atoms/preferences.ts apps/desktop/src/renderer/services/backend.ts
git commit -m "feat(rtk): add RTK renderer atom and backend service"
```

---

### Task 7: Settings UI — RTK Toggle

**Files:**
- Modify: `apps/desktop/src/renderer/components/settings/general-settings.tsx`

**Interfaces:**
- Consumes: `rtkEnabledAtom` from `../../atoms/preferences`
- Consumes: `checkRtk`, `enableRtk`, `disableRtk`, `installRtk` from `../../services/backend`

- [ ] **Step 1: Update imports in general-settings.tsx (line 13)**

Add `rtkEnabledAtom` to the import from atoms:

```ts
import { type DisplayMode, displayModeAtom, opaqueWindowsAtom, rtkEnabledAtom } from "../../atoms/preferences"
```

Add the backend service imports after line 16:

```ts
import { checkRtk, disableRtk, enableRtk, fetchOpenInTargets, installRtk, setOpenInPreferred } from "../../services/backend"
```

- [ ] **Step 2: Add RTK state interface and RtkRow component**

After the `OpencodeBinaryRow` component (after line 215), add:

```tsx
interface RtkState {
	checking: boolean
	installed: boolean
	version: string | null
	pluginActive: boolean
	enabling: boolean
	error: string | null
}

function RtkRow() {
	const enabled = useAtomValue(rtkEnabledAtom)
	const setEnabled = useSetAtom(rtkEnabledAtom)
	const [state, setState] = useState<RtkState>({
		checking: true,
		installed: false,
		version: null,
		pluginActive: false,
		enabling: false,
		error: null,
	})
	const pollRef = useRef<ReturnType<typeof setInterval> | null>(null)

	// Check RTK status on mount
	useEffect(() => {
		if (!isElectron) {
			setState((s) => ({ ...s, checking: false }))
			return
		}
		checkRtk().then((result) => {
			setState({
				checking: false,
				installed: result.installed,
				version: result.version,
				pluginActive: result.pluginActive,
				enabling: false,
				error: null,
			})
		})
	}, [])

	// Sync local atom with main process setting
	useEffect(() => {
		if (!isElectron) return
		return window.circulo.onSettingsChanged((settings) => {
			const rtkSetting = (settings as { rtkEnabled?: boolean }).rtkEnabled
			if (typeof rtkSetting === "boolean" && rtkSetting !== enabled) {
				setEnabled(rtkSetting)
			}
		})
	}, [enabled, setEnabled])

	// Cleanup poll on unmount
	useEffect(() => {
		return () => {
			if (pollRef.current) {
				clearInterval(pollRef.current)
			}
		}
	}, [])

	const handleToggle = useCallback(
		async (checked: boolean) => {
			setState((s) => ({ ...s, enabling: true, error: null }))
			try {
				if (checked) {
					const result = await enableRtk()
					if (result.success) {
						setEnabled(true)
						setState((s) => ({ ...s, enabling: false, pluginActive: true }))
					} else {
						setState((s) => ({
							...s,
							enabling: false,
							error: result.error ?? "Failed to enable RTK",
						}))
					}
				} else {
					const result = await disableRtk()
					if (result.success) {
						setEnabled(false)
						setState((s) => ({ ...s, enabling: false, pluginActive: false }))
					} else {
						setState((s) => ({ ...s, enabling: false, error: "Failed to disable RTK" }))
					}
				}
			} catch (err) {
				setState((s) => ({
					...s,
					enabling: false,
					error: err instanceof Error ? err.message : "Unknown error",
				}))
			}
		},
		[setEnabled],
	)

	const handleInstall = useCallback(async () => {
		await installRtk()
		// Poll for installation
		pollRef.current = setInterval(async () => {
			const result = await checkRtk()
			if (result.installed) {
				if (pollRef.current) {
					clearInterval(pollRef.current)
					pollRef.current = null
				}
				setState({
					checking: false,
					installed: true,
					version: result.version,
					pluginActive: result.pluginActive,
					enabling: false,
					error: null,
				})
			}
		}, 2000)
		// Stop polling after 60s
		setTimeout(() => {
			if (pollRef.current) {
				clearInterval(pollRef.current)
				pollRef.current = null
			}
			setState((s) => ({ ...s, checking: false, error: "Installation timeout" }))
		}, 60_000)
	}, [])

	if (!isElectron) return null

	const isLoading = state.checking || state.enabling

	return (
		<SettingsRow
			label="RTK Token Optimization"
			description="Reduce LLM token consumption by 60-90% by filtering command outputs before they reach the model. Requires the rtk CLI tool."
		>
			<div className="flex items-center gap-3">
				<Switch
					checked={enabled}
					onCheckedChange={handleToggle}
					disabled={!state.installed || isLoading}
				/>
				{isLoading && (
					<span className="text-xs text-muted-foreground">
						<Loader2Icon aria-hidden="true" className="inline size-3 animate-spin mr-1" />
						{state.enabling ? "Restarting server..." : "Checking..."}
					</span>
				)}
				{!isLoading && state.installed && (
					<span className="text-xs text-muted-foreground">
						{enabled
							? `Active — rtk v${state.version ?? "?"}`
							: `rtk v${state.version ?? "?"} detected`}
					</span>
				)}
				{!isLoading && !state.installed && (
					<button
						type="button"
						onClick={handleInstall}
						className="inline-flex items-center gap-1 rounded-md bg-primary px-3 py-1.5 text-xs font-medium text-primary-foreground hover:bg-primary/90 transition-colors"
					>
						Install RTK
					</button>
				)}
				{state.error && (
					<span className="text-xs text-destructive">{state.error}</span>
				)}
			</div>
		</SettingsRow>
	)
}
```

- [ ] **Step 3: Add Loader2Icon import**

Update the lucide-react import on line 11:

```ts
import { Loader2Icon, MonitorIcon, MoonIcon, SunIcon } from "lucide-react"
```

- [ ] **Step 4: Add useRef import**

Update the react import on line 12:

```ts
import { useCallback, useEffect, useRef, useState } from "react"
```

- [ ] **Step 5: Add RtkRow to GeneralSettings component**

Update the `GeneralSettings` component to include `RtkRow` in the Appearance section, after `OpaqueWindowsRow`:

```tsx
export function GeneralSettings() {
	return (
		<div className="space-y-8">
			<div>
				<h2 className="text-xl font-semibold">General</h2>
			</div>

			<SettingsSection>
				<OpenDestinationRow />
			</SettingsSection>

			<SettingsSection title="Appearance">
				<ThemeRow />
				{isMac && <OpaqueWindowsRow />}
				<RtkRow />
				<OpencodeBinaryRow />
				<DisplayModeRow />
			</SettingsSection>
		</div>
	)
}
```

- [ ] **Step 6: Verify types compile**

```bash
cd apps/desktop && bun run check-types
```

- [ ] **Step 7: Run lint fix**

```bash
cd apps/desktop && bunx biome check --write src/renderer/components/settings/general-settings.tsx
```

- [ ] **Step 8: Commit**

```bash
git add apps/desktop/src/renderer/components/settings/general-settings.tsx
git commit -m "feat(rtk): add RTK toggle to General Settings"
```

---

### Task 8: Integration Test — Verify End-to-End

- [ ] **Step 1: Run full type check**

```bash
cd apps/desktop && bun run check-types
```

Expected: No type errors

- [ ] **Step 2: Run lint check**

```bash
bunx biome check apps/desktop/src/
```

Expected: No errors (some warnings may exist from pre-existing code)

- [ ] **Step 3: Manual verification checklist**

- Build the desktop app: `cd apps/desktop && bun run dev`
- Open Settings → General
- Verify RTK toggle appears
- Verify "Install RTK" button is shown when `rtk` is not in PATH
- Click "Install RTK" → verify terminal opens with install command
- Manually install `rtk` binary → verify toggle enables automatically
- Toggle ON → verify OpenCode server restarts
- Toggle OFF → verify OpenCode server restarts

- [ ] **Step 4: Final commit (if any adjustments)**

```bash
git add -A
git commit -m "feat(rtk): final integration polish"
```
