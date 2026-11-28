import type { BrowserWindow } from "electron"
import type { Context } from "../index"

/**
 * Contexto tRPC cuando el backend corre como sidecar (sin ventana Electron real).
 */
export async function createContext(): Promise<Context> {
  return {
    getWindow: () => null as unknown as BrowserWindow,
  }
}