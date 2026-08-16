# AGENTS.md — Contrato de ingeniería de Circulo

Este archivo es de cumplimiento obligatorio para cualquier agente (humano o IA) que toque este repositorio.

Circulo es un cliente de escritorio nativo (GPUI + Rust) para personas de marketing, producto y diseño. La fuente de producto está en `docs/` y en `Circulo-Project-Definition.md`. El método de construcción es **OpenSpec**.

Si este archivo choca con un hábito del agente (“ser útil”, “adelantar trabajo”, “no molestar al usuario”), gana este archivo.

---

## 0. Identidad del trabajo

- Producto: Circulo.
- Plataforma MVP: macOS.
- UI: GPUI, componentes propios, animaciones nativas. Locale `en` (todas las cadenas en catálogos).
- Dos procesos Circulo: `circulo-app` (GPUI) y `circulo-daemon`.
- App ↔ daemon: contrato Circulo por HTTPS + SSE.
- Daemon ↔ OpenCode: HTTP + SSE de OpenCode, solo vía adapter. La UI no habla con OpenCode.
- Persistencia: SQLite. `session.project_id` es opcional (carpeta especial `Sessions`).
- Specs: OpenSpec (`openspec/`, schema `spec-driven`).
- Documentos de producto: `docs/PRD.md`, `docs/TRD.md`, `docs/UX-UI.md`, `docs/FLOWS.md`, `docs/IMPLEMENTATION.md`.

Leer esos documentos **antes** de proponer o escribir código.

---

## 1. Reglas de oro

### 1.1 Primero preguntar, nunca asumir

Si falta un dato que cambia alcance, comportamiento observable, compatibilidad, UX o criterio de aceptación: **parar y preguntar**.

Está prohibido:

- Elegir “lo más razonable” en silencio cuando el PRD/TRD/UX lo marcan como decisión abierta.
- Inventar copy, flujos, defaults de proyecto, persistencia, o APIs de OpenCode.
- Completar requisitos ambiguos para no interrumpir.

Asumir solo detalles menores de implementación interna que no escapen al usuario ni al contrato, y **dejarlos escritos** en el design del change.

### 1.2 Investigar antes de cambiar

Ningún cambio de código, spec o documento ocurre sin investigación previa:

- Leer el código y los docs que ya existen.
- Verificar APIs externas (OpenCode, GPUI, macOS) en la fuente vigente, no de memoria.
- Entender el impacto en otros módulos y flujos.

Si no investigaste, no editas.

### 1.3 Consultar y pedir permiso

Investigar no autoriza implementar.

Antes de crear archivos de producto, crates, dependencias, APIs o refactors:

1. Resumir el hallazgo.
2. Proponer el cambio mínimo.
3. Pedir permiso explícito.
4. Esperar.

El usuario pidiendo “piensa en X” o “define Y” **no** es permiso para implementar X. El usuario pidiendo “implementa el change Z” o “aplica OpenSpec” sí lo es, y solo para Z.

Excepción: este repositorio autoriza mantener OpenSpec/docs cuando el usuario pidió expresamente esos artefactos. El código de la app no está autorizado por este archivo.

### 1.4 OpenSpec es el único camino de construcción

Siempre construimos con spec. Flujo:

| Intención | Skill / comando |
| --- | --- |
| Explorar, dudar, aclarar | `openspec-explore` / `/opsx-explore` |
| Proponer un change | `openspec-propose` / `/opsx-propose` |
| Revisar/actualizar el plan | `openspec-update-change` / `/opsx-update` |
| Implementar tasks | `openspec-apply-change` / `/opsx-apply` — solo tras permiso |
| Archivar | `openspec-archive-change` / `/opsx-archive` — solo tras permiso |
| Sincronizar specs | `openspec-sync-specs` / `/opsx-sync` |

No hay features “fuera de banda”. No hay “hotfix rápido” que salte el spec salvo que el usuario lo autorice y se documente después en el mismo ciclo.

Un change OpenSpec = una feature = un branch.

### 1.5 Modular siempre

- Un crate / módulo / componente, una responsabilidad.
- El frontend no conoce OpenCode.
- Un adapter nuevo es un crate nuevo, no un `match` gigante en el daemon.
- No crear frameworks internos preventivos.
- No copiar tipos entre capas: viven en `core` / `protocol`.
- Preferir componer módulos pequeños a inflar un archivo “god”.

### 1.6 Las pruebas importan más que complacer

Hacer que el proyecto funcione bien es mejor que decir que sí.

Si el usuario pide algo que rompe un principio de producto, el modelo de datos, la modularidad, o la calidad del stream:

- Explicar el costo.
- Ofrecer una alternativa más sana.
- No implementarlo “para quedar bien”.

Un agente que entrega código que compila y está mal diseñado **falló**, aunque el usuario esté contento en el momento.

### 1.7 Clean code y prácticas de app nativa

- Nombres precisos. Funciones cortas. Sin efectos ocultos.
- Errores tipados; no `unwrap()` en caminos de producción.
- Sin estado global improvisado.
- UI: estados vacío / carga / error / éxito. Nada a medias.
- Performance percibida: el thread de UI no se bloquea con parseo ni IO.
- No comentarios que narran lo obvio. Sí comentarios que fijan un invariante no evidente.
- No código muerto, no TODOs eternos, no “por si acaso”.
- Dependencias: mínimas, justificadas, pedidas con permiso.

### 1.8 Git: branch por feature, commits granulares, commit después de prueba manual

**Branches**

- `main` se mantiene estable.
- Cada feature/change: `feature/<openspec-change-name>`.
- No mezclar dos features en un branch.

**Commits**

- Un propósito por commit.
- Conventional Commits (`feat:`, `fix:`, `test:`, `refactor:`, `docs:`, `chore:`).
- Mensajes que describen el cambio, no el estado de ánimo.

**Cuándo commitear**

1. El slice está implementado.
2. Tests automáticos relevantes pasan.
3. **Prueba manual hecha** si el cambio es observable (UI, API, flujo).
4. Recién entonces `git commit`.

Está prohibido commitear para “guardar progreso” sin validar. Está prohibido un commit único con todo el change si el change tiene partes separables.

Nunca `git push --force` a `main`. Nunca commitear secretos, `.env`, ni datos locales de usuario.

El agente **no commitea** a menos que el usuario lo pida después de las pruebas, o el change en curso lo autorice de forma explícita. En caso de duda: preguntar.

---

## 2. Ingeniería de software — estándar Circulo

### 2.1 Antes de escribir código

- [ ] ¿Hay un change OpenSpec con specs y tasks?
- [ ] ¿Leí los docs de producto que aplican?
- [ ] ¿Investigué el código y las APIs reales?
- [ ] ¿Hay decisiones abiertas que este cambio resolvería en silencio?
- [ ] ¿Tengo permiso para este slice?

Si algún recuadro es no: no escribes código.

### 2.2 Diseño

- Separar dominio, contrato, persistencia, adapter y UI.
- Invertir dependencias hacia traits (`AgentAdapter`), no hacia OpenCode.
- Cambios breaking del protocolo = versionado o change explícito.
- Datos: UUIDs, JSON serde estable, status machines explícitas.

### 2.3 UI (GPUI)

- Componentes según `docs/UX-UI.md`. No inventar una jerarquía paralela.
- Dark theme. Animaciones nativas solamente.
- Sin title bar nativo; traffic lights y hide/show alineados en el Sidebar (rail al colapsar).
- Copy humano en inglés, sin jerga de CLI en primer nivel. **Cero literales de UI** fuera de locale files.
- New session → sin proyecto, label “No project”. No agrupar la lista salvo acción manual del usuario.
- Verificar el flujo a mano, no solo “se renderiza”.

### 2.4 Tests

Cada change que toque lógica trae tests en la misma capa.

Mínimo aceptable:

- Transiciones de estado (message/tool/session).
- Roundtrip JSON del protocolo.
- Persistencia: CRUD y que una sesión no aparezca en el proyecto equivocado.
- Adapter: mapping con fixtures; unreachable → error humano mapeable.
- Daemon: integración con adapter fake.

La UI se prueba a mano siguiendo `docs/FLOWS.md`.

### 2.5 Definition of Done (código)

Hecho significa:

1. Cumple el spec del change, no una interpretación amplia.
2. Tests pasan.
3. Prueba manual del flujo tocado.
4. Sin decisiones abiertas cerradas a escondidas.
5. Diff acotado al permiso dado.
6. Docs/specs actualizados si el comportamiento cambió de forma deliberada y aprobada.

---

## 3. Qué no hacer

- No implementar el MVP de un tirón.
- No introducir Electron, Tauri, React o WebView como chat principal.
- No hablar con OpenCode desde la UI.
- No añadir proveedores extra “porque es fácil”.
- No implementar `QuestionCard` en el MVP.
- No crear un sistema de plugins genérico.
- No añadir telemetría.
- No “mejorar” el alcance para impresionar.
- No editar `openspec/specs/` a mano para reflejar un change no archivado, salvo el flujo oficial de sync/archive.
- No mencionar estas reglas de forma teatral al usuario en cada mensaje. Cumplirlas.

---

## 4. Decisiones abiertas y cerradas

Listas vivas: `docs/PRD.md` §12 y `docs/TRD.md` §15. Tratar las abiertas como **bloqueos**.

Ya cerrado (no reabrir sin preguntar):

- Traffic lights + hide alineados en el Sidebar; rail mínimo al colapsar.
- Sesión nueva sin proyecto, carpeta especial `Sessions` (`project_id` null).
- Agrupación solo manual. Item: nombre, tiempo activa, “No project” o nombre de proyecto.
- SQLite.
- Dos procesos Circulo. OpenCode es externo.
- UI en inglés con infraestructura de locales.

---

## 5. Comunicación con el usuario

- Preguntas concretas, una decisión por pregunta cuando sea posible.
- Distinguir hecho investigado vs opinión.
- Decir cuando algo no se verificó.
- No inflar el progreso. “Compila” no es “funciona”.
- Español si el usuario escribe en español, salvo que pida otra cosa.

---

## 6. Mapa rápido

```
Circulo-Project-Definition.md    idea original (v0.5)
docs/PRD.md                      qué y por qué
docs/TRD.md                      cómo (arquitectura)
docs/UX-UI.md                    superficie
docs/FLOWS.md                    flujos y estados
docs/IMPLEMENTATION.md           orden de changes
openspec/                        specs ejecutables
AGENTS.md                        este contrato
```
