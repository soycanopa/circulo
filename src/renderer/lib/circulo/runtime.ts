export function isTauriRuntime(): boolean {
  if (typeof window === "undefined") return false
  return !!(window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__
}

export function isElectronRuntime(): boolean {
  if (typeof window === "undefined") return false
  // trpc-electron expone ipcLink; desktopApi viene del preload Electron
  return !isTauriRuntime() && typeof (window as Window & { desktopApi?: unknown }).desktopApi !== "undefined"
}

export const SIDECAR_TRPC_URL =
  import.meta.env.VITE_CIRCULO_TRPC_URL ?? "http://127.0.0.1:21337"