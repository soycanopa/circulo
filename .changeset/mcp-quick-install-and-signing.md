---
"@circulo/desktop": minor
---

Mejora la instalación rápida de MCP con tipos remote/local-http y activa el code
signing + notarización de macOS en los releases de CI.

- MCP: la instalación rápida soporta plantillas `remote` y `local-http` con
  `serverUrl` / `localUrl`. Simplifica el flujo de instalación y limpia la UI
  del template card.
- macOS: los builds de release ahora se firman con Developer ID Application y se
  notarizan automáticamente cuando los secrets `APPLE_*` / `MAC_CSC_*` están
  configurados. Esto desbloquea el auto-update real (quitAndInstall) en macOS,
  reemplazando el fallback de "descargar del navegador".
