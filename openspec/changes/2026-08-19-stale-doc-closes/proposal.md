# stale-doc-closes

## Why

The previous change (`2026-08-19-cleanup-and-doc-sync`) closed the open decisions in PRD §12 and TRD §15. Several stale "decisiones abiertas" mentions remain in other docs that were not touched by that change:

- `docs/TRD.md:165` (TRD-API-02) still calls HTTPS a "decisión abierta" though §15.1 already records it as resolved.
- `docs/TRD.md:253-259` lists five "Decisiones abiertas del adapter" though all five were resolved by `opencode-provider-hardening` and the §15 sync.
- `docs/UX-UI.md:347-352` lists four "decisiones abiertas de UX" of which three are resolved by the current UI and only one remains genuinely open (inline title editing, deferred).
- `docs/FLOWS.md:77` still describes session title generation as "P1 y abierta" though `opencode-provider-hardening` §6 closed it (default titles only, manual renames preserved).

Closing these stale mentions keeps the docs honest about what is and isn't actually open.

## What Changes

Pure markdown edits. No code, no spec deltas.

### Closed by code (rewrite as resolved, not "open")

| Doc | Section | Resolution |
| --- | --- | --- |
| `docs/TRD.md` | §6.1 TRD-API-02 | HTTP plano en localhost; TLS diferido fuera de MVP (mismo que §15.1). |
| `docs/TRD.md` | §7.2 (lista 1–5) | Reescrita como resumen de implementación: bundled spawn on :7433; session mapping via `agent_session_id`; cwd implícito desde project folder; subset de eventos = todos los del `EVENTS.md`; auth diferida con attach mode. |
| `docs/UX-UI.md` | §11 items 1, 3, 4 | Cerradas: Enter envía (UX §4.11), Settings es panel dentro de la ventana, densidad de `SessionItem` implementada (nombre + carpeta/duración). |
| `docs/FLOWS.md` | §5 (nota sobre título) | Auto-título vía OpenCode sobre default titles; renombres manuales no se pisan (`opencode-provider-hardening` §6). |

### Still genuinely open (kept)

- `docs/UX-UI.md` §11 item 2: "¿El título se edita inline o en un diálogo?" — fuera del MVP. P1.
- `docs/PRD.md` §11 Multi-agente — apunta a v0.3.

## Capabilities

### Modified Capabilities

(none — pure docs sync)

### New Capabilities

(none)

## Impact

- Solo Markdown. Cero cambios en código Rust.
- Sin migraciones. Sin nuevas dependencias. Sin cambios de protocolo.

## Non-goals

- Cerrar PRD §11 Multi-agente (debe esperar a v0.3).
- Implementar el inline title editing.
- Cambiar el spec de `app-shell` (ya está actualizado en `cleanup-and-doc-sync`).
- Modificar `docs/POST-MVP.md` (no hay nada stale que valga la pena cerrar allí).

## Open questions

(none)