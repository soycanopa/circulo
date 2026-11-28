/**
 * Implementación parcial de `window.desktopApi` sobre comandos Tauri.
 * Se amplía módulo a módulo según docs/MIGRACION.md.
 */
import { invoke } from "@tauri-apps/api/core"
import { getCurrentWindow } from "@tauri-apps/api/window"
import { openUrl } from "@tauri-apps/plugin-opener"
import { writeText, readText } from "@tauri-apps/plugin-clipboard-manager"
import type { DesktopApi } from "../../../preload/index"
import { SIDECAR_TRPC_URL } from "./runtime"

const noopUnsub = () => {}

const unsupported = (name: string) => () => {
  console.warn(`[circulo] desktopApi.${name} aún no migrado (ver MIGRACION.md)`)
  return Promise.resolve(undefined)
}

let cachedInfo: { version: string; platform: string; arch: string } | null = null

async function loadInfo() {
  if (!cachedInfo) {
    const info = await invoke<{
      version: string
      platform: string
      arch: string
    }>("app_info")
    cachedInfo = info
  }
  return cachedInfo
}

function mapPlatform(os: string): NodeJS.Platform {
  if (os === "macos") return "darwin"
  if (os === "windows") return "win32"
  if (os === "linux") return "linux"
  return os as NodeJS.Platform
}

export async function buildTauriDesktopApi(): Promise<DesktopApi> {
  const info = await loadInfo()
  const platform = mapPlatform(info.platform)
  const win = getCurrentWindow()

  const api = {
    platform,
    arch: info.arch,
    getVersion: async () => info.version,
    isPackaged: async () => import.meta.env.PROD,

    // Auto-update (pendiente en Tauri)
    checkForUpdates: unsupported("checkForUpdates"),
    downloadUpdate: unsupported("downloadUpdate"),
    installUpdate: () => unsupported("installUpdate")(),
    setUpdateChannel: unsupported("setUpdateChannel"),
    getUpdateChannel: async () => "latest",
    onUpdateChecking: () => noopUnsub,
    onUpdateAvailable: () => noopUnsub,
    onUpdateNotAvailable: () => noopUnsub,
    onUpdateProgress: () => noopUnsub,
    onUpdateDownloaded: () => noopUnsub,
    onUpdateError: () => noopUnsub,
    onUpdateManualCheck: () => noopUnsub,

    windowMinimize: () => invoke("window_minimize"),
    windowMaximize: () => invoke("window_maximize"),
    windowClose: () => invoke("window_close"),
    windowIsMaximized: () => invoke("window_is_maximized"),
    windowToggleFullscreen: () => invoke("window_toggle_fullscreen"),
    windowIsFullscreen: () => invoke("window_is_fullscreen"),
    setTrafficLightVisibility: unsupported("setTrafficLightVisibility"),
    setWindowFramePreference: unsupported("setWindowFramePreference"),
    getWindowFrameState: async () => false,
    onFullscreenChange: (callback) => {
      const unlisten = win.onResized(async () => {
        callback(await invoke<boolean>("window_is_fullscreen"))
      })
      void unlisten
      return noopUnsub
    },
    onFocusChange: (callback) => {
      const offFocus = win.onFocusChanged(({ payload }) => callback(payload))
      void offFocus
      return noopUnsub
    },
    zoomIn: unsupported("zoomIn"),
    zoomOut: unsupported("zoomOut"),
    zoomReset: unsupported("zoomReset"),
    getZoom: async () => 0,
    newWindow: unsupported("newWindow"),
    setWindowTitle: (title: string) => invoke("window_set_title", { title }),
    claimChat: unsupported("claimChat"),
    releaseChat: unsupported("releaseChat"),
    focusChatOwner: unsupported("focusChatOwner"),
    toggleDevTools: async () => {
      await win.openDevTools()
    },
    unlockDevTools: unsupported("unlockDevTools"),
    setAnalyticsOptOut: unsupported("setAnalyticsOptOut"),
    setBadge: unsupported("setBadge"),
    setBadgeIcon: unsupported("setBadgeIcon"),
    showNotification: unsupported("showNotification"),
    openExternal: async (url: string) => {
      await openUrl(url)
    },
    getApiBaseUrl: async () => "https://api.circulo.dev",
    clipboardWrite: async (text: string) => {
      await writeText(text)
    },
    clipboardRead: async () => readText(),
    saveFile: unsupported("saveFile"),
    getUser: unsupported("getUser"),
    isAuthenticated: unsupported("isAuthenticated"),
    logout: unsupported("logout"),
    startAuthFlow: unsupported("startAuthFlow"),
    submitAuthCode: unsupported("submitAuthCode"),
    updateUser: unsupported("updateUser"),
    getAuthToken: unsupported("getAuthToken"),
    signedFetch: unsupported("signedFetch"),
    streamFetch: unsupported("streamFetch"),
    onStreamChunk: () => noopUnsub,
    onStreamDone: () => noopUnsub,
    onStreamError: () => noopUnsub,
    onAuthSuccess: () => noopUnsub,
    onAuthError: () => noopUnsub,
    onShortcutNewAgent: () => noopUnsub,
    onShortcutOpenSettings: () => noopUnsub,
    onFileChanged: () => noopUnsub,
    onGitStatusChanged: () => noopUnsub,
    onWorktreeSetupFailed: () => noopUnsub,
    subscribeToGitWatcher: unsupported("subscribeToGitWatcher"),
    unsubscribeFromGitWatcher: unsupported("unsubscribeFromGitWatcher"),
    scanVSCodeThemes: unsupported("scanVSCodeThemes"),
    loadVSCodeTheme: unsupported("loadVSCodeTheme"),
  }

  return api as DesktopApi
}