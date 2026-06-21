---
"@circulo/desktop": minor
"@circulo/ui": minor
---

Ajustes de UI en el sidebar y la zona del chat en dark mode.

- Elimina la zona "Active Now" del sidebar; los chats activos viven únicamente
  dentro de su carpeta de proyecto. La carpeta del proyecto se auto-expande al
  iniciar un chat nuevo.
- En dark mode, el área de mensajes y el composer del chat usan un fondo sólido
  #161616 uniforme (sin costura), con la caja de texto distinguible. Se
  reactivan los fades de scroll (solo en dark) y se elimina la línea divisoria.
- Light mode y tier opaco (Windows/Linux/browser) sin cambios.
