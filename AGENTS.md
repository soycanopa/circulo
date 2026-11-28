# AGENTS.md — Círculo

Reglas para cualquier agente (o persona) que trabaje en este repositorio.

**Enfoque actual (2026-03):** cliente de escritorio **Rust + Tauri v2** con UI
**React/TypeScript** heredada de [1Code](https://github.com/21st-dev/1code) (Apache-2.0).
No es un fork upstream: no hay PRs a 21st-dev/1code. El producto se llama **Círculo**;
no uses marcas ni logotipos de 1Code/21st.dev.

**Referencia histórica:** la línea anterior (Rust + GPUI) está en `legacy/circulo/`.

## Acuerdo de trabajo

- Sé directo y técnico. Documenta en español salvo que el código o el estándar exijan inglés.
- Explica el *porqué* con documentación al proponer cambios.
- Si una decisión humana contradice buenas prácticas o la barra de calidad, alza la mano.
- **No inventes APIs.** Lee docs oficiales de Tauri, del proveedor de agente y `docs/`.
- **No asumas. Pregunta** cuando el producto, permisos o protocolo no estén claros.
- **Nada grande sin plan** — issue, commit, o `docs/` para trabajo amplio.
- **No entres en bucle.** Tras dos intentos fallidos, documenta y cambia de estrategia.
- **No simplifiques ni elimines features** durante la migración salvo petición explícita.
- **Código heredado 1Code:** conserva Apache-2.0 en `LICENSE-1CODE` y `NOTICE`; no copies GPL de proyectos de referencia.

## OpenSpec (heredado de 1Code)

Si el cambio habla de propuesta, spec, arquitectura o capacidad nueva, consulta
`openspec/AGENTS.md` y el flujo de tres fases (propuesta → implementación → archivo).

## Arquitectura objetivo

Separación en capas; dependencias en una sola dirección.

| Capa | Responsabilidad | Notas |
|------|-----------------|-------|
| `src/renderer/` | UI React, estado Jotai/Zustand, tRPC cliente | Sin lógica de agentes en disco |
| `src-tauri/` | Shell nativo, permisos, spawn, futuro core Rust | Comandos Tauri tipados |
| `src/main/` | Backend heredado (migración gradual) | Sidecar Node mientras dure |
| `sidecar/` | Puente HTTP tRPC | **Temporal** — ver `docs/MIGRACION.md` |
| `legacy/circulo/` | Solo lectura / referencia | No editar salvo preservación |

**Conflictos de reglas:** manda lo definido para **Círculo** en este archivo y en `legacy/circulo/AGENTS.md`.

## Flujo Git

- `main` protegido — **todo cambio entra por PR** (incluido el maintainer).
- Rama: `feature/<tema>`, `fix/<tema>`, `docs/<tema>`.
- Commits: `tipo(ámbito): resumen` (español o inglés consistente por PR).
- Un cambio lógico por commit; preferir commits pequeños; **no squash** al mergear.
- Tras merge, borrar la rama.

### Checks locales (camino Tauri)

```bash
CIRCULO_TAURI=1 bun install
bun run build:ui
cargo fmt --all --check    # en src-tauri
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

## Estándares de código

- TypeScript estricto en UI; Rust idiomático en `src-tauri/`.
- Errores explícitos; sin abstracciones prematuras.
- No bloquear el hilo de UI; trabajo pesado en Rust async o sidecar.
- Eventos desconocidos de proveedores: ignorar con gracia (compatibilidad).

## Mapa del repo

- `docs/MIGRACION.md` — estado módulo a módulo y hoja de ruta semver
- `docs/` — documentación de producto (mezcla heredada + nueva)
- `src/renderer/` — interfaz agentes, terminal, sidebar, settings
- `src/main/lib/trpc/routers/` — API backend (migrar a Rust)
- `src-tauri/src/commands/` — comandos nativos ya migrados
- `NOTICE`, `LICENSE` (MIT Círculo), `LICENSE-1CODE` (Apache 1Code)

## Barra de calidad

- Streaming perceptible como en 1Code.
- Transcripts largos fluidos.
- Sin procesos de agente huérfanos al cerrar.
- Nuevos proveedores o vistas deben añadirse sin reescribir el resto.