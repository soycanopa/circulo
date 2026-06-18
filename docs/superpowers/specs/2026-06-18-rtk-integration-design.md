# RTK Integration Design

**Date**: 2026-06-18
**Status**: Approved

## Overview

Integrate [RTK](https://github.com/rtk-ai/rtk) (Rust Token Killer) into Circulo. RTK is a CLI proxy that reduces LLM token consumption by 60-90% by filtering and compressing command outputs before they reach the LLM.

## Architecture

Circulo manages RTK transparently. The user sees only a toggle; Circulo handles the underlying OpenCode plugin lifecycle internally.

```
Renderer (Toggle UI) ──IPC──► Main Process (rtk-service.ts)
                                    │
                          ┌─────────┼──────────┐
                          ▼         ▼          ▼
                    isInstalled?  plugin.ts  restartServer()
```

### Components

1. **`main/rtk/plugin-source.ts`** — Embeds the 22-line OpenCode plugin as a string constant. Circulo writes/removes this file at `~/.config/opencode/plugins/rtk.ts` under demand.

2. **`main/rtk/rtk-service.ts`** — Service in the main process:
   - `isRtkInstalled()` — spawns `which rtk`
   - `getRtkVersion()` — spawns `rtk --version`
   - `installPlugin()` — writes `rtk.ts` to OpenCode plugins dir
   - `uninstallPlugin()` — removes `rtk.ts` from OpenCode plugins dir
   - `isPluginActive()` — checks if `rtk.ts` exists

3. **`renderer/atoms/preferences.ts`** — New atom: `rtkEnabledAtom` (`atomWithStorage`, key `"circulo:rtkEnabled"`, default `false`).

4. **`main/settings-store.ts`** — New field `rtkEnabled?: boolean` in `AppSettings`.

5. **`renderer/components/settings/general-settings.tsx`** — New "Token Optimization" section with:
   - Toggle switch to enable/disable RTK
   - Status badge showing installed version or "not installed"
   - "Install RTK" button (when not installed) that opens terminal

6. **IPC Channels**:
   - `rtk:check` — returns `{ installed, version? }`
   - `rtk:enable` — writes plugin, persists setting, restarts OpenCode
   - `rtk:disable` — removes plugin, persists setting, restarts OpenCode
   - `rtk:install` — opens terminal with install command

## User Flows

### Flow 1: RTK already installed
1. User opens Settings → General
2. Sees toggle "Activar RTK" with green badge "rtk vX.Y.Z detectado"
3. Activates toggle
4. Circulo writes plugin, restarts OpenCode
5. Toggle ON, badge shows "Activo"

### Flow 2: RTK not installed
1. User opens Settings → General
2. Sees disabled toggle with amber badge "RTK no instalado"
3. Clicks "Instalar RTK"
4. Circulo opens terminal with `curl -fsSL ... | sh`
5. Circulo polls every 2s for `rtk` in PATH (max 60s timeout)
6. Once detected, toggle enables automatically
7. User activates → Flow 1

### Flow 3: Disable
1. User deactivates toggle
2. Circulo removes plugin, restarts OpenCode
3. Toggle OFF

## Edge Cases

- **RTK uninstalled while toggle ON**: On app start, detect missing binary → disable toggle, show warning
- **Server restart slow**: Show spinner while restarting
- **Plugin already exists** (manually installed): Detect, sync toggle state
- **Repeated polling failure**: After 60s timeout, stop polling, show error state

## Data Flow

```
Settings Page → rtkEnabledAtom (localStorage, instant UI)
             → IPC rtk:check → main process spawn check
             → IPC rtk:enable/disable → main: write/delete plugin
                                     → main: restart OpenCode
                                     → main: persist to settings.json
```

## How RTK Filtering Works

The OpenCode plugin intercepts `tool.execute.before` events. When the LLM generates a bash command (e.g., `git status`), the plugin runs `rtk rewrite "git status"`, receives `rtk git status`, and mutates the command before execution. The RTK binary then filters the output.

Only `bash` and `shell` tools are intercepted. OpenCode built-in tools (`Read`, `Grep`, `Glob`) are not affected.
