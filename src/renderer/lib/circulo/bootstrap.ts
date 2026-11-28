import { buildTauriDesktopApi } from "./desktop-api-tauri"
import { isTauriRuntime } from "./runtime"

/**
 * Debe importarse antes que el resto de la UI en entorno Tauri.
 */
export async function installCirculoTauriBridge(): Promise<void> {
  if (!isTauriRuntime()) return
  if (window.desktopApi) return
  window.desktopApi = await buildTauriDesktopApi()
  console.log("[circulo] desktopApi Tauri instalado")
}

void installCirculoTauriBridge()