import react from "@vitejs/plugin-react"
import { resolve } from "path"
import { defineConfig } from "vite"

const isDev = process.env.NODE_ENV !== "production"

export default defineConfig({
  root: resolve(__dirname, "src/renderer"),
  publicDir: resolve(__dirname, "src/renderer/public"),
  envPrefix: ["VITE_", "CIRCULO_"],
  plugins: [
    react({
      jsxImportSource: isDev
        ? "@welldone-software/why-did-you-render"
        : undefined,
    }),
  ],
  resolve: {
    alias: {
      "@": resolve(__dirname, "src/renderer"),
    },
  },
  server: {
    host: "127.0.0.1",
    port: 1420,
    strictPort: true,
  },
  build: {
    outDir: resolve(__dirname, "dist"),
    emptyOutDir: true,
    rollupOptions: {
      input: {
        index: resolve(__dirname, "src/renderer/index.html"),
        login: resolve(__dirname, "src/renderer/login.html"),
      },
    },
  },
})