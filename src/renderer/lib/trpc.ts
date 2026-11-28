import { createTRPCReact } from "@trpc/react-query"
import { createTRPCProxyClient } from "@trpc/client"
import type { AppRouter } from "../../main/lib/trpc/routers"
import { createCirculoTrpcLink } from "./circulo/trpc-link"

/**
 * React hooks for tRPC
 */
export const trpc = createTRPCReact<AppRouter>()

/**
 * Vanilla client for use outside React components (stores, utilities)
 */
export const trpcClient = createTRPCProxyClient<AppRouter>({
  links: [createCirculoTrpcLink()],
})
