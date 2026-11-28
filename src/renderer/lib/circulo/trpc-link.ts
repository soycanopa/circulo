import { httpBatchLink } from "@trpc/client"
import type { TRPCLink } from "@trpc/client"
import { ipcLink } from "trpc-electron/renderer"
import superjson from "superjson"
import type { AppRouter } from "../../../main/lib/trpc/routers"
import { isTauriRuntime, SIDECAR_TRPC_URL } from "./runtime"

export function createCirculoTrpcLink(): TRPCLink<AppRouter> {
  if (isTauriRuntime()) {
    return httpBatchLink({
      url: `${SIDECAR_TRPC_URL}/`,
      transformer: superjson,
      headers: () => ({
        "x-circulo-client": "tauri",
      }),
    }) as TRPCLink<AppRouter>
  }

  return ipcLink({ transformer: superjson }) as TRPCLink<AppRouter>
}