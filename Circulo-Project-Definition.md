# Circulo — Project Definition

**Versión del documento:** 0.6  
**Fecha:** 16 de agosto de 2026  
**Estado:** Idea / Pre-MVP  
**Nombre del proyecto:** Circulo

---

## 1. Visión del Producto

**Circulo** es un cliente de escritorio de alto rendimiento diseñado para orquestar agentes de IA a través de una interfaz de chat moderna, limpia y rica.

No está pensado principalmente para desarrolladores expertos, sino para **personas de marketing, producto, diseño y perfiles no técnicos** que quieren usar agentes de IA para crear cosas de calidad sin tener que pelearse con herramientas complejas.

### Filosofía de Producto

> **"Las herramientas suficientes para hacer el trabajo genial."**

- No queremos 100 mil funcionalidades.
- Queremos las que realmente permiten **crear**, y que funcionen muy bien.
- Preferimos profundidad y calidad sobre cantidad de features.
- La experiencia debe sentirse accesible, clara y poderosa al mismo tiempo.

Circulo busca ser el lugar donde estas personas pueden conversar con agentes de IA de forma fluida, organizar su trabajo por proyectos y obtener resultados estructurados y útiles, sin necesidad de conocimientos técnicos profundos.

---

## 2. Público Objetivo

| Perfil              | Descripción                                                                 | Necesidad principal                              |
|---------------------|-----------------------------------------------------------------------------|--------------------------------------------------|
| Marketing           | Personas que crean campañas, contenidos, landings, etc.                     | Generar y refinar textos, ideas y assets rápido  |
| Producto            | PMs y product owners que definen features y flujos                          | Explorar ideas, estructurar requisitos, prototipar |
| Diseño              | Diseñadores que trabajan con copy, estructura y a veces código ligero       | Apoyo creativo + generación de contenido         |
| No-técnicos en general | Cualquier persona que quiere usar agentes de IA sin ser desarrollador    | Interfaz simple, resultados claros, sin fricción |

**Nota:** Aunque el producto puede ser útil para desarrolladores, el diseño de la experiencia prioriza la claridad y la accesibilidad para perfiles no técnicos.

---

## 3. Problema que resuelve

- Las herramientas actuales de agentes de IA están pensadas principalmente para developers y suelen ser complejas o visualmente pobres.
- El streaming de respuestas suele ser texto plano, difícil de seguir y poco estructurado.
- No hay una forma simple y visual de organizar el trabajo por proyectos cuando se usan agentes.
- Muchas soluciones consumen muchos recursos (Electron) y se sienten lentas.

Circulo busca ofrecer una experiencia **rápida, limpia y bien diseñada** orientada a crear, no a configurar.

---

## 4. Principios de Diseño

1. **Claridad sobre complejidad** — La interfaz debe ser comprensible para alguien sin conocimientos técnicos.
2. **Herramientas suficientes, no excesivas** — Cada feature debe justificar su existencia por el valor que aporta al acto de crear.
3. **Velocidad y responsividad** como prioridad absoluta.
4. **Streaming de alta calidad** — La respuesta del agente debe verse estructurada y bonita *mientras* se genera.
5. **Organización por proyecto** — El trabajo se agrupa de forma natural.
6. **Modularidad** — La arquitectura permite crecer sin volverse un monstruo.
7. **Experiencia visual cuidada** — Animaciones nativas, tipografía limpia, componentes bien resueltos.

---

## 5. Funcionalidades Principales

### 5.1 Interfaz de Chat Rica

El chat debe soportar un stream estructurado que pueda renderizar:

- **Markdown de alta calidad** (código, tablas, listas, encabezados, etc.).
- **Componente de Tareas (Tasks)**: la IA puede definir tareas con estado (pendiente, en progreso, completada).
- **Componente de Preguntas interactivas**: la IA puede hacer preguntas al usuario de forma estructurada (opciones, input libre, confirmaciones).
- **Tool Calls visuales**:
  - Cards bonitas por cada tool call.
  - Diffs renderizados en tiempo real cuando el agente edita o genera contenido.
  - Indicadores claros de estado (ejecutando, completado, error).
- **Otros elementos**: resultados de búsquedas, imágenes, archivos, etc.
- El contenido debe ir apareciendo de forma organizada a medida que el agente responde (live stream).

### 5.2 Organización de Sesiones

- Panel derecho (o lateral) con las sesiones.
- Las sesiones se pueden **agrupar por proyecto**.
- Posibilidad de crear proyectos y **asignar** sesiones a ellos a mano.
- Una sesión nueva nace sin proyecto (carpeta especial `Sessions`).
- Vista clara de qué agente está usando cada sesión.
- Búsqueda y filtrado de sesiones.

### 5.3 Orquestación de Agentes

- Soporte para múltiples agentes a través de un sistema de **adapters/plugins**.
- Cada proveedor se implementa como un módulo independiente.
- Comunicación con los agentes vía **HTTPS + SSE** a través de un daemon local.

### 5.4 Daemon / Backend Local

- Proceso daemon simple que se ejecuta en segundo plano.
- **Responsabilidades en esta etapa:**
  - Permitir conversar con la IA sin fricción.
  - Gestionar la conexión con el agente (inicialmente OpenCode).
  - Exponer una API local (HTTPS + SSE) al frontend.
  - Manejar el streaming de respuestas.
- Se mantiene deliberadamente simple. Funciones avanzadas se agregarán solo cuando sean necesarias.

---

## 6. Alcance del MVP (Versión Inicial)

### Objetivo del MVP
Tener una aplicación usable, rápida y bonita que permita a usuarios no técnicos **conversar de forma rica con OpenCode**, organizar su trabajo por proyectos y obtener respuestas bien estructuradas.

### Incluido en el MVP

| Área                    | Qué se incluye                                      | Qué se deja fuera                          |
|-------------------------|-----------------------------------------------------|--------------------------------------------|
| Proveedores             | Solo OpenCode                                       | Cursor, Claude Code, Grok, Codex, etc.     |
| Chat                    | Markdown + Tool Calls + Diffs + Tasks básicos       | Preguntas interactivas avanzadas           |
| Sesiones                | Lista plana; sesión nueva sin proyecto; agrupación solo manual | Colaboración, compartición, etc. |
| Arquitectura            | App + daemon (2 procesos) + adapter OpenCode + SQLite | Sistema de plugins completo      |
| UI                      | Diseño limpio + animaciones nativas de GPUI         | Temas avanzados, personalización profunda  |
| Plataforma              | macOS (prioridad)                                   | Windows / Linux (después)                  |

### Criterios de éxito del MVP
- Una persona de marketing o producto puede abrir Circulo y conversar con OpenCode sin fricción.
- El streaming se siente estructurado y agradable.
- Se pueden organizar sesiones por proyecto de forma intuitiva.
- La aplicación se siente rápida y responsiva.
- La arquitectura permite agregar el siguiente proveedor sin reescribir todo.

---

## 7. Arquitectura Técnica

```
┌─────────────────────────────────────────────────────┐
│         circulo-app (GPUI - Rust)  proceso 1        │
│  - Chat stream rico                                 │
│  - Sidebar de sesiones                              │
│  - Componentes visuales propios                     │
└──────────────────────┬──────────────────────────────┘
                       │ HTTPS + SSE (contrato Circulo)
                       ▼
┌─────────────────────────────────────────────────────┐
│        circulo-daemon (Rust)  proceso 2             │
│  - API local + persistencia SQLite                  │
│  - Streaming + orquestación                         │
└──────────────────────┬──────────────────────────────┘
                       │ HTTP + SSE (API OpenCode)
                       ▼
              ┌──────────────────┐
              │ OpenCode server  │  proceso externo
              └──────────────────┘
```

### Principios de arquitectura
- **Todo es modular**: los adapters de agentes se diseñan como plugins desde el inicio.
- El frontend no conoce los detalles de cada CLI; solo habla con el daemon.
- El daemon se mantiene **simple** en esta etapa (solo lo necesario para conversar bien).
- Comunicación: HTTPS + SSE para streaming en tiempo real.

---

## 8. Stack Tecnológico (Decisiones tomadas)

| Capa              | Tecnología elegida            | Notas                                              |
|-------------------|-------------------------------|----------------------------------------------------|
| Frontend UI       | **GPUI (Rust)**               | Decisión tomada. Componentes propios.               |
| Estilos           | Sistema propio + inspiración en Tailwind/Shadcn | Se construirán los componentes necesarios         |
| Iconos            | Heroicons                     |                                                    |
| Animaciones       | **Animaciones nativas de GPUI** | No se usará Anime.js                               |
| Runtime / Tooling | Bun + TypeScript              | Para scripts y tooling                             |
| Backend / Daemon  | Rust                          | Mantenerlo simple al inicio                        |
| Comunicación      | HTTPS + SSE                   |                                                    |
| Modularidad       | Adapters / plugins            |                                                    |

**Sobre los componentes de UI:**  
Al usar GPUI nativo no existe un ecosistema grande de componentes listos. Se construirán los necesarios, tomando inspiración de implementaciones en otros lenguajes y frameworks (Shadcn, Base UI, etc.).

---

## 9. Experiencia de Usuario Deseada

- La aplicación se siente **instantánea** y ligera.
- Una persona sin conocimientos técnicos puede usarla sin sentirse perdida.
- Al recibir respuesta del agente, el contenido aparece de forma ordenada y con buen ritmo.
- Los tool calls se ven como tarjetas elegantes y comprensibles.
- Los diffs (cuando existan) se pueden revisar cómodamente.
- El panel de sesiones permite cambiar de contexto por proyecto de forma natural.
- Las animaciones son nativas, sutiles y fluidas.
- Ventana moderna sin title bar nativo: los controles de ventana (traffic lights) y el botón de colapsar sidebar viven integrados en el propio Sidebar.

---

## 10. Roadmap de Alto Nivel

1. **MVP** → OpenCode + Chat rico + Sesiones por proyecto + Daemon simple
2. **v0.2** → Mejoras de usabilidad basadas en feedback de usuarios no técnicos
3. **v0.3** → Agregar 1 proveedor más (evaluar según demanda)
4. **v0.4** → Preguntas interactivas + mejor soporte de Tasks
5. **v0.5** → Sistema de adapters más maduro
6. **Futuro** → Más agentes, posibles flujos multi-agente, etc.

---

## 11. Riesgos y Decisiones Tomadas / Abiertas

| Tema                        | Estado                          | Notas                                              |
|----------------------------|---------------------------------|----------------------------------------------------|
| GPUI vs Tauri              | **Decidido: GPUI**              | Se construyen componentes propios                  |
| Animaciones                | **Decidido: nativas de GPUI**   |                                                    |
| Title bar / Window controls| **Decidido: sin title bar nativo** | Traffic lights + botón colapsar viven dentro del Sidebar |
| Complejidad del daemon     | **Decidido: simple al inicio**  | Solo lo necesario para conversar bien              |
| Procesos                   | **Decidido: app + daemon**      | OpenCode es proceso externo vía HTTP/SSE           |
| Persistencia               | **Decidido: SQLite**            | `project_id` opcional                              |
| Sesión nueva               | **Decidido: sin proyecto**      | Carpeta especial Sessions; label “No project”      |
| Agrupación                 | **Decidido: solo manual**       | Lista inicial plana                                |
| Idioma UI                  | **Decidido: inglés + locales**  | Listo para más idiomas                             |
| Público objetivo           | **Definido**                    | Marketing, producto, diseño, no-técnicos           |
| Filosofía de producto      | **Definida**                    | Herramientas suficientes que funcionen muy bien    |
| Soporte de muchos agentes  | Abierto                         | MVP solo OpenCode, luego ir de a uno               |
| Diferencias entre CLIs     | Abierto                         | Abstracción fuerte en los adapters                 |

---

## 12. Modelo de Datos

Este es el modelo de datos propuesto para Circulo. Está pensado para ser simple, extensible y fácil de serializar (JSON) entre el daemon y el frontend.

### 12.1 Entidades principales

```
Project
 └── Session[]
      └── Message[]
           ├── parts: MessagePart[]
           └── (metadatos)
```

---

### 12.2 Project

Representa un contenedor lógico de trabajo (una campaña, un producto, un diseño, etc.).

```rust
struct Project {
    id: String,                    // UUID
    name: String,
    description: Option<String>,
    color: Option<String>,         // Para identificación visual (ej: "#6366f1")
    created_at: DateTime,
    updated_at: DateTime,
    // sessions se cargan por separado o se referencian por id
}
```

---

### 12.3 Session

Una conversación con un agente, perteneciente a un proyecto.

```rust
struct Session {
    id: String,                    // UUID
    project_id: Option<String>,    // None = carpeta especial Sessions (sin proyecto)
    title: String,                 // Generado automáticamente o editable
    agent: AgentType,              // Enum: OpenCode | ClaudeCode | Cursor | ...
    status: SessionStatus,         // Active | Archived | Error
    created_at: DateTime,
    updated_at: DateTime,
    last_message_at: Option<DateTime>,
}
```

```rust
enum AgentType {
    OpenCode,
    // Futuros:
    // ClaudeCode,
    // Cursor,
    // Grok,
    // Codex,
    // ...
}

enum SessionStatus {
    Active,
    Archived,
    Error,
}
```

---

### 12.4 Message

Un mensaje dentro de una sesión. Puede ser del usuario o del agente.

```rust
struct Message {
    id: String,                    // UUID
    session_id: String,            // FK → Session
    role: MessageRole,             // User | Assistant | System
    parts: Vec<MessagePart>,       // Contenido estructurado
    status: MessageStatus,         // Streaming | Complete | Error
    created_at: DateTime,
    // Para streaming:
    is_streaming: bool,
}
```

```rust
enum MessageRole {
    User,
    Assistant,
    System,
}

enum MessageStatus {
    Pending,
    Streaming,
    Complete,
    Error,
}
```

---

### 12.5 MessagePart (el corazón del chat rico)

En lugar de tener un solo campo `content: String`, el mensaje se compone de **partes**. Esto permite mezclar texto, tool calls, tareas, preguntas, etc. de forma limpia.

```rust
enum MessagePart {
    Text {
        content: String,           // Markdown
    },
    ToolCall {
        tool_call: ToolCall,
    },
    TaskList {
        tasks: Vec<Task>,
    },
    Question {
        question: Question,
    },
    // Futuros posibles:
    // Image { url: String, alt: Option<String> },
    // File { name: String, path: String, ... },
}
```

---

### 12.6 ToolCall

Representa una llamada a herramienta realizada por el agente.

```rust
struct ToolCall {
    id: String,                    // ID único de la tool call
    name: String,                  // Ej: "edit_file", "web_search", "run_command"
    status: ToolCallStatus,        // Pending | Running | Success | Error
    input: serde_json::Value,      // Argumentos de la tool (JSON)
    output: Option<ToolOutput>,    // Resultado (cuando termina)
    started_at: Option<DateTime>,
    finished_at: Option<DateTime>,
}

enum ToolCallStatus {
    Pending,
    Running,
    Success,
    Error,
}

enum ToolOutput {
    Text { content: String },
    Diff {
        file_path: String,
        old_content: Option<String>,
        new_content: String,
        // o un formato de diff unificado
        diff: Option<String>,
    },
    Json { data: serde_json::Value },
    Error { message: String },
}
```

---

### 12.7 Task

Componente de tareas que la IA puede generar.

```rust
struct Task {
    id: String,
    title: String,
    description: Option<String>,
    status: TaskStatus,            // Pending | InProgress | Completed | Cancelled
    order: u32,                    // Para mantener el orden
}

enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Cancelled,
}
```

---

### 12.8 Question (Preguntas interactivas)

Para cuando la IA necesita input del usuario de forma estructurada.

```rust
struct Question {
    id: String,
    prompt: String,                // La pregunta
    question_type: QuestionType,
    options: Option<Vec<String>>,  // Si es de selección
    answer: Option<String>,        // Respuesta del usuario (cuando responde)
    status: QuestionStatus,        // Pending | Answered | Skipped
}

enum QuestionType {
    TextInput,                     // Input libre
    SingleSelect,                  // Una opción
    MultiSelect,                   // Varias opciones
    Confirm,                       // Sí / No
}

enum QuestionStatus {
    Pending,
    Answered,
    Skipped,
}
```

---

### 12.9 Relaciones y notas de diseño

- **Project 0..1 → N Session** (`project_id` opcional)
- **Session 1 → N Message**
- **Message 1 → N MessagePart**
- Un `Message` del asistente puede contener varias `MessagePart` (texto + tool calls + tasks, etc.).
- Durante el streaming, un `Message` puede ir recibiendo nuevas `MessagePart` o actualizando las existentes (especialmente ToolCalls).
- Los IDs son UUIDs para facilitar sincronización y evitar colisiones.
- Todo debe ser serializable a JSON de forma limpia (importante para la comunicación daemon ↔ frontend).

---

### 12.10 Ejemplo de un Message completo (JSON simplificado)

```json
{
  "id": "msg_abc123",
  "session_id": "ses_xyz",
  "role": "assistant",
  "status": "complete",
  "parts": [
    {
      "type": "text",
      "content": "He analizado el copy de la landing. Aquí tienes las mejoras:"
    },
    {
      "type": "task_list",
      "tasks": [
        {
          "id": "task_1",
          "title": "Reescribir el titular principal",
          "status": "completed"
        },
        {
          "id": "task_2",
          "title": "Ajustar el CTA del hero",
          "status": "in_progress"
        }
      ]
    },
    {
      "type": "tool_call",
      "tool_call": {
        "id": "tc_001",
        "name": "edit_file",
        "status": "success",
        "input": { "path": "landing.md", "content": "..." },
        "output": {
          "type": "diff",
          "file_path": "landing.md",
          "diff": "..."
        }
      }
    }
  ]
}
```

---

## 13. Estructura de Componentes de UI

Basado en la referencia visual (Waku) y en el modelo de datos de Circulo.

### 13.1 Layout General de la Aplicación

**Decisión de ventana:**  
No se usa title bar nativo. La aplicación usa **hidden title bar** (o titlebar-less) y los **traffic lights** (cerrar, minimizar, maximizar) viven **dentro del Sidebar**, junto con el botón para ocultar/mostrar el sidebar.

```
┌────────────────────────────────────────────────────────────────────┐
│ ┌──────────────────┐                                               │
│ │ ● ● ●  [≡]       │  Session Header                               │
│ │                  │  (título + acciones)                          │
│ │  New Session     ├───────────────────────────────────────────────┤
│ │  Search          │                                               │
│ │                  │                                               │
│ │  Session List    │           Messages Area                       │
│ │  (plana)         │           (scrollable)                        │
│ │                  │                                               │
│ │                  │                                               │
│ │                  ├───────────────────────────────────────────────┤
│ │  Settings        │  Composer                                     │
│ └──────────────────┘  (input + controles)                          │
└────────────────────────────────────────────────────────────────────┘
     ↑
 Traffic lights + botón de colapsar sidebar
 viven dentro del Sidebar
```

---

### 13.2 Componentes de Alto Nivel

| Componente              | Responsabilidad                                      | Datos que consume          |
|-------------------------|------------------------------------------------------|----------------------------|
| `AppShell`              | Layout principal (sidebar + main). Maneja ventana sin title bar nativo | -                    |
| `Sidebar`               | Navegación + traffic lights + botón colapsar         | Projects + Sessions        |
| `SessionHeader`         | Título de la sesión + acciones                       | Session                    |
| `MessagesArea`          | Lista de mensajes con scroll                         | Message[]                  |
| `Message`               | Un mensaje completo (user o assistant)               | Message                    |
| `MessagePartRenderer`   | Decide qué componente renderizar según el tipo       | MessagePart                |
| `Composer`              | Input + selectores + botón enviar                    | -                          |
| `StatusBar` (opcional)  | Info de proyecto / rama / estado                     | Session + Project          |

---

### 13.3 Sidebar

```
Sidebar
├── SidebarTopBar
│   ├── TrafficLights          (● ● ●) — controles de ventana
│   └── CollapseSidebarButton  (icono para ocultar/mostrar sidebar)
├── SidebarHeader
│   ├── NewSessionButton
│   └── SearchInput
├── SessionList
│   ├── SessionGroup           (ej: "Yesterday", "This Month")
│   │   └── SessionItem        (título, proyecto, tiempo relativo)
│   └── SessionGroup
│       └── SessionItem
└── SidebarFooter
    └── SettingsButton
```

**Notas de implementación (macOS + GPUI):**
- La ventana se configura con title bar oculto / transparent title bar.
- Los traffic lights se posicionan manualmente en la esquina superior izquierda del Sidebar.
- El botón de colapsar sidebar queda junto a los traffic lights.
- Cuando el sidebar está colapsado, se debe decidir si los traffic lights migran al `SessionHeader` o se mantiene un sidebar mínimo.

**SessionItem** muestra:
- Título de la sesión
- Tiempo activa relativo (`16m`, `14h`, `2d`)
- Proyecto: nombre definido por el usuario, o **No project**
- Estado activo (highlight)

Las sesiones nuevas van a la carpeta especial de sistema `Sessions` (sin proyecto). La lista no se agrupa sola; el usuario asigna un proyecto a mano si quiere. Traffic lights y el botón de ocultar permanecen alineados en el Sidebar (rail mínimo al colapsar). UI en inglés, todas las cadenas en locales.

---

### 13.4 Messages Area & Message

```
MessagesArea
└── Message[] 
    ├── UserMessage
    │   └── TextPart (o parts simples)
    └── AssistantMessage
        ├── MessageMeta (ej: "Worked for 10 seconds")
        └── MessagePart[] 
            ├── TextPart
            ├── ToolCallCard
            ├── TaskList
            └── QuestionCard
```

#### TextPart
- Renderiza Markdown de alta calidad.
- Soporte para inline code con estilo de “pill” (como los highlights naranjas de la referencia).
- Headings, listas, negritas, etc.

#### ToolCallCard
Componente visual importante. Muestra:
- Nombre de la tool
- Estado (Pending / Running / Success / Error) con indicador visual
- Input resumido o expandable
- Output:
  - Si es `Diff` → renderizado de diff bonito
  - Si es texto → bloque de código o texto
  - Si es error → mensaje de error claro

#### TaskList
- Lista de `Task` con checkbox / estado visual
- Título + estado (Pending, In Progress, Completed)
- Posibilidad de interacción futura (marcar como hecha)

#### QuestionCard
- Pregunta clara
- Según `QuestionType`:
  - TextInput
  - Botones de SingleSelect / MultiSelect
  - Confirm (Sí / No)
- Estado: Pending → Answered

---

### 13.5 Composer (Input inferior)

```
Composer
├── TextInput (multiline)
├── ComposerToolbar
│   ├── AgentSelector (por ahora solo OpenCode)
│   ├── ModeSelector (opcional)
│   └── SendButton
└── ComposerFooter (opcional)
    └── Project / Branch info
```

En el MVP se mantiene simple:
- Input de texto
- Selector de agente (aunque solo haya uno)
- Botón de enviar
- Indicador de que está generando (cuando `is_streaming`)

---

### 13.6 Principios de diseño visual (inspirados en la referencia)

- **Dark theme** por defecto (elegante y enfocado).
- Tipografía limpia y buena jerarquía.
- Inline code y paths con estilo “pill” / badge.
- Espaciado generoso pero no excesivo.
- Estados claros (activo, streaming, error).
- Animaciones nativas sutiles:
  - Aparición de mensajes
  - Transición de estados de ToolCall
  - Hover en SessionItem
- El contenido del asistente se siente estructurado, no como un muro de texto.

---

### 13.7 Mapeo Modelo de Datos → Componentes

| Modelo de Datos     | Componente(s) que lo renderizan      |
|---------------------|--------------------------------------|
| `Project`           | `SessionItem` (nombre), posible ProjectSwitcher |
| `Session`           | `SessionItem`, `SessionHeader`       |
| `Message`           | `Message` / `UserMessage` / `AssistantMessage` |
| `MessagePart::Text` | `TextPart`                           |
| `MessagePart::ToolCall` | `ToolCallCard`                   |
| `MessagePart::TaskList` | `TaskList`                       |
| `MessagePart::Question` | `QuestionCard`                   |
| `ToolCall`          | `ToolCallCard`                       |
| `Task`              | Item dentro de `TaskList`            |
| `Question`          | `QuestionCard`                       |

---

## 14. Próximos Pasos Recomendados

1. ~~Definir el **modelo de datos**~~ → Hecho
2. ~~Definir la **estructura de componentes visuales**~~ → Hecho
3. Diseñar la **interfaz del Adapter** de OpenCode
4. Empezar a prototipar los componentes clave en GPUI (especialmente `TextPart`, `ToolCallCard` y `SessionItem`)
5. Implementar el daemon mínimo

---

## Resumen Ejecutivo

**Circulo** es un cliente de escritorio nativo (GPUI + Rust) orientado a personas de marketing, producto y diseño.  

Su filosofía es clara: **las herramientas suficientes para hacer el trabajo genial**.  

Se enfoca en una experiencia de chat rica y estructurada, organización por proyectos y una arquitectura modular, comenzando con un MVP simple centrado en OpenCode y un daemon ligero.

La interfaz se inspira en referencias como Waku: sidebar limpia con sesiones agrupadas, área de mensajes con contenido estructurado y un composer simple pero poderoso.  
**No usa title bar nativo**: los traffic lights y el botón de colapsar sidebar viven dentro del propio Sidebar.

---

*Documento actualizado — Versión 0.6 (sesiones sin proyecto, SQLite, dos procesos, i18n)*
