/**
 * Sustituto de `electron` para el sidecar Círculo (sin runtime Electron).
 * Se activa vía override en package.json → `npm:bun` install.
 */
const { EventEmitter } = require("node:events")
const { existsSync, mkdirSync } = require("node:fs")
const { homedir } = require("node:os")
const { join } = require("node:path")

const userData = process.env.CIRCULO_USER_DATA ?? join(homedir(), ".circulo-dev")
const workspaceRoot = process.env.CIRCULO_WORKSPACE ?? join(__dirname, "../..")

if (!existsSync(userData)) {
  mkdirSync(userData, { recursive: true })
}

class BrowserWindow extends EventEmitter {
  id = 1
  webContents = {
    send: () => {},
    on: () => {},
    off: () => {},
    openDevTools: () => {},
    session: { webRequest: { onHeadersReceived: () => {} } },
  }
  show() {}
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
  maximize() {}
  unmaximize() {}
  setTitle() {}
  setWindowButtonVisibility() {}
  focus() {}
}

const app = {
  getName: () => "Círculo",
  getVersion: () => "0.1.0",
  isPackaged: false,
  getPath: (name) => {
    const map = {
      userData,
      home: homedir(),
      temp: join(userData, "tmp"),
      appData: userData,
      exe: process.execPath,
    }
    return map[name] ?? join(userData, name)
  },
  getAppPath: () => workspaceRoot,
  getLocale: () => "es",
  on: (event, cb) => {
    if (event === "ready") setImmediate(cb)
  },
  whenReady: () => Promise.resolve(),
  quit: () => process.exit(0),
  requestSingleInstanceLock: () => true,
  commandLine: { appendSwitch: () => {} },
  dock: { setBadge: () => {}, bounce: () => 0 },
  setPath: () => {},
}

Object.defineProperty(process, "resourcesPath", { value: workspaceRoot })

module.exports = {
  app,
  BrowserWindow,
  clipboard: { writeText: () => {}, readText: () => "" },
  contextBridge: { exposeInMainWorld: () => {} },
  dialog: {
    showOpenDialog: async () => ({ canceled: true, filePaths: [] }),
    showSaveDialog: async () => ({ canceled: true }),
  },
  ipcMain: { handle: () => {}, on: () => {}, removeHandler: () => {} },
  ipcRenderer: { invoke: async () => undefined, on: () => {}, removeListener: () => {} },
  Menu: { setApplicationMenu: () => {}, buildFromTemplate: () => ({}) },
  nativeImage: { createFromPath: () => ({ isEmpty: () => true }) },
  nativeTheme: { shouldUseDarkColors: true, on: () => {}, off: () => {} },
  Notification: { isSupported: () => true },
  safeStorage: {
    isEncryptionAvailable: () => false,
    encryptString: (s) => Buffer.from(s),
    decryptString: (b) => b.toString(),
  },
  screen: { getPrimaryDisplay: () => ({ workAreaSize: { width: 1920, height: 1080 } }) },
  session: { defaultSession: { webRequest: { onHeadersReceived: () => {} } } },
  shell: { openExternal: async () => {}, openPath: async () => "" },
  webFrame: { setZoomLevel: () => {}, getZoomLevel: () => 0 },
  webUtils: { getPathForFile: () => "" },
  systemPreferences: { getColor: () => "#000000" },
  powerMonitor: { on: () => {} },
}