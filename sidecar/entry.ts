/**
 * Entrada del sidecar: expone el router tRPC heredado vía HTTP para el webview Tauri.
 */
import { createHTTPServer } from "@trpc/server/adapters/standalone"
const PORT = Number(process.env.CIRCULO_SIDECAR_PORT ?? 21337)

async function main() {

  let status = "starting"
  let dbMessage: string | null = null

  try {
    const { initDatabase } = await import("../src/main/lib/db/index.ts")
    initDatabase()
    dbMessage = "ok"
    status = "db-ready"
  } catch (error) {
    dbMessage = error instanceof Error ? error.message : String(error)
    console.warn("[sidecar] initDatabase falló (esperado mientras migramos módulos):", error)
    status = "degraded"
  }

  let appRouter: import("../src/main/lib/trpc/routers").AppRouter | null = null
  try {
    const { createAppRouter } = await import("../src/main/lib/trpc/routers/index.ts")
    const { BrowserWindow } = await import("electron")
    const getWindow = () => new BrowserWindow()
    appRouter = createAppRouter(getWindow)
    status = status === "degraded" ? "router-partial" : "router-ready"
  } catch (error) {
    console.warn("[sidecar] createAppRouter falló:", error)
    status = "health-only"
  }

  if (appRouter) {
    const { createContext } = await import(
      "../src/main/lib/trpc/routers/index-sidecar.ts"
    )
    const server = createHTTPServer({
      router: appRouter,
      createContext,
    })

    server.listen(PORT, "127.0.0.1", () => {
      console.log(`[sidecar] tRPC HTTP en http://127.0.0.1:${PORT} estado=${status}`)
    })
    return
  }

  // Fallback mínimo si el router completo no puede cargarse todavía
  const http = await import("node:http")
  http
    .createServer((_req, res) => {
      res.writeHead(200, { "Content-Type": "application/json" })
      res.end(
        JSON.stringify({
          service: "circulo-sidecar",
          status,
          db: dbMessage,
          trpc: false,
        }),
      )
    })
    .listen(PORT, "127.0.0.1", () => {
      console.log(`[sidecar] health HTTP en http://127.0.0.1:${PORT}`)
    })
}

main().catch((error) => {
  console.error("[sidecar] fatal", error)
  process.exit(1)
})