/**
 * Mock mínimo de APIs Electron usadas por el backend heredado de 1Code.
 * Solo para el sidecar Tauri — NO es un reemplazo completo del runtime Electron.
 */
import { EventEmitter } from "node:events"
import { existsSync, mkdirSync } from "node:fs"
import { homedir } from "node:os"
import { dirname, join } from "node:path"

const userData =
  process.env.CIRCULO_USER_DATA ?? join(homedir(), ".circulo-dev")
const workspaceRoot = process.env.CIRCULO_WORKSPACE ?? process.cwd()

function ensureDir(path: string) {
  if (!existsSync(path)) {
    mkdirSync(path, { recursive: true })
  }
}

ensureDir(userData)

export class BrowserWindow extends EventEmitter {
  id = 1
  webContents = {
    send: (_channel: string, _payload?: unknown) => {},
    on: () => {},
    off: () => {},
    openDevTools: () => {},
    session: {
      webRequest: {
        onHeadersReceived: () => {},
      },
    },
  }

  constructor(_options?: unknown) {
    super()
  }

  loadURL() {
    return Promise.resolve()
  }
  loadFile() {
    return Promise.resolve()
  }
  show() {}
  hide() {}
  close() {}
  isDestroyed() {
    return false
  }
  isMaximized() {
    return false
  }
  isFullScreen() {
    return false
  }
  setFullScreen() {}
  minimize() {}
  maximize() {}
  unmaximize() {}
  setTitle() {}
  setWindowButtonVisibility() {}
  getBounds() {
    return { x: 0, y: 0, width: 1400, height: 900 }
  }
  focus() {}
}

const nativeThemeEmitter = new EventEmitter()

export const nativeTheme = {
  shouldUseDarkColors: true,
  on: (event: string, cb: () => void) => nativeThemeEmitter.on(event, cb),
  off: (event: string, cb: () => void) => nativeThemeEmitter.off(event, cb),
}

export const session = {
  defaultSession: {
    webRequest: {
      onHeadersReceived: () => {},
    },
  },
}

export const dialog = {
  showOpenDialog: async () => ({ canceled: true, filePaths: [] as string[] }),
  showSaveDialog: async () => ({ canceled: true, filePath: undefined as string | undefined }),
}

export const shell = {
  openExternal: async (url: string) => {
    console.log("[sidecar] shell.openExternal", url)
  },
  openPath: async () => "",
}

export const Menu = {
  setApplicationMenu: () => {},
  buildFromTemplate: () => ({}),
}

export const Notification = {
  isSupported: () => true,
}

export const nativeImage = {
  createFromPath: () => ({ isEmpty: () => true }),
}

export const app = {
  getName: () => "Círculo",
  getVersion: () => process.env.npm_package_version ?? "0.1.0",
  isPackaged: false,
  getPath: (name: string) => {
    switch (name) {
      case "userData":
        return userData
      case "home":
        return homedir()
      case "temp":
        return join(userData, "tmp")
      case "appData":
        return userData
      case "exe":
        return process.execPath
      default:
        return join(userData, name)
    }
  },
  getAppPath: () => workspaceRoot,
  getLocale: () => "es",
  on: (event: string, cb: (...args: unknown[]) => void) => {
    if (event === "ready") {
      setImmediate(cb)
    }
  },
  whenReady: () => Promise.resolve(),
  quit: () => process.exit(0),
  requestSingleInstanceLock: () => true,
  setPath: () => {},
  commandLine: { appendSwitch: () => {} },
  dock: {
    setBadge: () => {},
    bounce: () => 0,
  },
}

export const ipcMain = {
  handle: () => {},
  on: () => {},
  removeHandler: () => {},
}

export const clipboard = {
  writeText: (text: string) => {
    // noop en sidecar; Tauri maneja portapapeles en el renderer bridge
    console.log("[sidecar] clipboard.writeText", text.slice(0, 32))
  },
  readText: () => "",
}

export const safeStorage = {
  isEncryptionAvailable: () => false,
  encryptString: (s: string) => Buffer.from(s),
  decryptString: (b: Buffer) => b.toString(),
}

export const contextBridge = {
  exposeInMainWorld: () => {},
}

export const ipcRenderer = {
  invoke: async () => undefined,
  on: () => {},
  removeListener: () => {},
}

export const webUtils = {
  getPathForFile: () => "",
}

export const webFrame = {
  setZoomLevel: () => {},
  getZoomLevel: () => 0,
}

export const systemPreferences = {
  getColor: () => "#000000",
}

export const screen = {
  getPrimaryDisplay: () => ({
    workAreaSize: { width: 1920, height: 1080 },
  }),
}

export const powerMonitor = {
  on: () => {},
}

// better-sqlite3 migrations en dev
Object.defineProperty(process, "resourcesPath", {
  value: workspaceRoot,
})

export function registerElectronMock() {
  const Module = require("node:module") as typeof import("node:module")
  const original = Module.prototype.require
  const electronApi = {
    app,
    BrowserWindow,
    clipboard,
    contextBridge,
    dialog,
    ipcMain,
    ipcRenderer,
    Menu,
    nativeImage,
    nativeTheme,
    Notification,
    safeStorage,
    screen,
    session,
    shell,
    systemPreferences,
    webFrame,
    webUtils,
    powerMonitor,
  }

  Module.prototype.require = function (id: string, ...args: unknown[]) {
    if (id === "electron") {
      return electronApi
    }
  return (original as (...a: unknown[]) => unknown).apply(this, [id, ...args])
  }

  // Migrations path fix: drizzle está en la raíz del repo
  const migrations = join(workspaceRoot, "drizzle")
  console.log("[sidecar] mock electron activo; userData=", userData, "migrations=", migrations)
  return { userData, workspaceRoot, migrationsDir: migrations }
}