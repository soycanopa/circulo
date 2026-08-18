# Circulo — Post-MVP roadmap notes

| Campo | Valor |
| --- | --- |
| Versión | 0.1 |
| Fecha | 18 de agosto de 2026 |
| Estado | Notas de planificación; no autoriza implementación |

Este documento captura trabajo **después** del change `mvp-hardening` y decisiones sobre scope creep ya presente en `main`.

---

## 1. Roadmap de producto (PRD §10)

| Versión | Foco |
| --- | --- |
| v0.2 | Usabilidad desde feedback de no técnicos |
| v0.3 | Segundo proveedor de agente |
| v0.4 | Preguntas interactivas + Tasks mejorados |
| v0.5 | Sistema de adapters maduro |

Cada incremento = un change OpenSpec propio.

---

## 2. Changes OpenSpec diferidos o pendientes

| Change | Motivo | Prioridad sugerida |
| --- | --- | --- |
| `opencode-attachments` | Rich prompt parts / imágenes en composer (hardening §8) | Después de feedback MVP |
| Attach mode (`CIRCULO_OPENCODE_ATTACH`) | Conectar a `opencode serve` externo (hardening §10) | Power-user; baja |
| Session archive UI | FLOWS §14 — sin undo en MVP | v0.2 o nunca |
| Rename project desde Settings | Fuera de `mvp-hardening` | v0.2 |

---

## 3. Scope ya en `main` fuera del MVP original

Documentar explícitamente para no re-debatir:

| Feature | Ubicación | Decisión |
| --- | --- | --- |
| QuestionCard / AskUserQuestion | `crates/circulo-app/src/ui/question_card.rs` | Mantener; ampliar en v0.4 con spec propio |
| Panel Models en Settings | `settings/models.rs` | Mantener; resolver PRD §12.3 (alcance Settings) |
| Permission modes en composer | `composer/` | Mantener; alinea con supervised tools |
| Activity clusters / layout polish | `shell.rs`, `ui/` | Mantener; no bloquea MVP |

---

## 4. Decisiones abiertas restantes

- **PRD §12.3:** ¿Settings final = General + Projects + Archived + Models, o recortar Models?
- **PRD §12.5:** Confirmar formalmente orden `last_message_at` desc en spec.
- **TRD §15:** HTTPS app↔daemon, virtualización de mensajes, macOS mínimo, bundle ID — investigar cuando toque plataforma.

---

## 5. Validación

El MVP se considera **feature-complete** tras `mvp-hardening` archivado. La calidad se valida de forma incremental por el usuario (sin pasada manual bloqueante del agente).
