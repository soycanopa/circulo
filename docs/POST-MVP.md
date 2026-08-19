# Circulo — Post-MVP roadmap notes

| Campo | Valor |
| --- | --- |
| Versión | 0.2 |
| Fecha | 19 de agosto de 2026 |
| Estado | Notas de planificación; no autoriza implementación |

Este documento captura trabajo **después** de que el MVP quedó feature-complete. Reemplaza la versión 0.1 (18-ago) y refleja que `mvp-hardening` ya está archivado.

---

## 1. Roadmap de producto (PRD §10)

| Versión | Foco |
| --- | --- |
| MVP | Feature-complete (archivado el 18-ago-2026 con `mvp-hardening`) |
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
| Panel Models en Settings | `crates/circulo-app/src/settings/models.rs` | Mantener; canónico en MVP (PRD §12.3 cerrado) |
| Permission modes en composer | `crates/circulo-app/src/composer/` | Mantener; alinea con supervised tools |
| Activity clusters / layout polish | `crates/circulo-app/src/ui/`, `shell.rs` | Mantener; no bloquea MVP |

---

## 4. Decisiones cerradas al sync (19-ago-2026)

| Ítem | Resolución |
| --- | --- |
| PRD §12.3 — alcance Settings | General + Projects + Archived + Models. Canónico en MVP. |
| PRD §12.5 — orden sidebar | `last_message_at DESC`, NULL al final, empate por `created_at DESC`. Spec actualizado. |
| TRD §15.1 — HTTPS local | HTTP plano en localhost; TLS diferido fuera de MVP. |
| TRD §15.2 — lanzamiento OpenCode | Bundled spawn por el daemon en puerto 7433; attach diferido. |
| TRD §15.3 — mapping session | `agent_session_id` persistido por sesión. |
| TRD §15.4 — pipeline Heroicons | `crates/circulo-app/src/icons.rs`. |
| TRD §15.5 — virtualización | `content_rail` + caché de markdown. Suficiente para MVP. |
| TRD §15.6 — macOS mínimo | macOS 14 (Sonoma). |
| TRD §15.7 — Bundle ID | `app.circulo.client`. |
| TRD §15.8 — i18n | JSON con fallback `en`. |
| TRD §15.9 — supervisión app↔daemon | `scripts/run-app.sh` + healthcheck `/v1/health`. |

---

## 5. Validación

El MVP se considera **feature-complete** desde el archivo de `mvp-hardening` (18-ago-2026). Las 12 tareas de verificación manual que quedaron abiertas en los changes archivados (`opencode-provider-hardening`, `composer-input-rebuild`, `live-session-stream`) son responsabilidad del usuario, no del agente; este último no commitea slices observables sin esa validación.

Una pasada manual completa de `docs/FLOWS.md` con OpenCode corriendo sigue siendo deseable antes de declarar "release candidate", pero no bloquea más cambios de producto.
