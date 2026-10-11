# Pulse: especificación

Pulse es una app de escritorio para Windows que muestra, fuera de la terminal, qué está haciendo Claude Code: luz en los bordes de la pantalla, una isla arriba al centro, un panel en la bandeja y un tablero completo. Se alimenta de los hooks de Claude Code.

Este documento recoge lo que se decidió en la conversación de diseño del 2026-10-10 y 2026-10-11. El plan de implementación (`docs/plans/2026-10-11-pulse.md`) se apoya en él.

## Referencias visuales

Los artboards son la fuente de verdad del aspecto y del movimiento. Hay copia local en `docs/artboards/`:

| Pieza | Artboard en vivo | Copia local |
|---|---|---|
| Isla (Comando y Orbe) | https://claude.ai/artifact/3evEKeuY2pKTunDWi8JX7K | `docs/artboards/isla.html` |
| Centro de mando (Bolsillo y Mosaico) | https://claude.ai/artifact/B5e5kr2seJdM8xf4fjurgv | `docs/artboards/centro-de-mando.html` |

Al portar una pieza, se copia su CSS y su lógica de animación desde el artboard (motor de pulsos, `morph()`, resortes con `linear()`), no se reinventa.

## Decisiones

### Arquitectura
- **Tauri v2 (Rust + WebView2).** Interfaz en HTML/CSS/TypeScript, lógica en Rust.
- **Interfaz mixta (decidido el 2026-10-11, durante la implementación).** Bordes e isla en TypeScript sin framework: son capas de animación y se portan tal cual desde los artboards. Bolsillo y Mosaico en React + Tailwind: son interfaces con tarjetas, pestañas, ajustes y piezas, y es donde más va a crecer el producto. Las cuatro ventanas comparten `bridge.ts` (tipos y llamadas al núcleo).
- **Arranque bajo demanda.** El primer hook lanza la app si no está corriendo. La app se cierra sola 5 minutos después de que no queda ninguna sesión activa ni eventos. No se registra en el inicio de Windows.
- **Hooks → `pulse.exe`.** Un ejecutable pequeño de Rust recibe el JSON del hook por stdin y lo manda a la app por un named pipe (`\\.\pipe\pulse`). Si la app no responde, la lanza y reintenta.
- **Permisos desde fuera de la terminal.** El hook `PermissionRequest` llama a `pulse ask`, que espera la decisión de la isla hasta un tiempo límite. Si nadie responde, no decide nada y Claude pregunta en la terminal como siempre.
- **Varias sesiones y subagentes.** Cada evento trae `session_id`; la app lleva el estado por sesión.

### Eventos y su significado

| Evento de Pulse | Hook de Claude Code | Color por defecto | Forma del pulso |
|---|---|---|---|
| `start` | `UserPromptSubmit` | `#e6eeff` | Respiración |
| `permission` | `PermissionRequest` | `#ffb02e` | Latido doble, deja brillo encendido |
| `done` | `Stop` | `#4fe0a0` | Ola perimetral |
| `idle` | `Notification` (`idle_prompt`) | `#b6c0d4` | Respiración lenta, deja brillo encendido |
| `error` | `PostToolUseFailure` | `#ff5a52` | Destello lateral |
| `subagent` | `SubagentStop` | `#5cc8ff` | Desde la terminal, sin sonido |
| `tool` | `PreToolUse` | sin pulso | alimenta la actividad y el Ticker |
| `session-start` / `session-end` | `SessionStart` / `SessionEnd` | sin pulso | cuenta de sesiones y autocierre |

### Pulsos en los bordes
- Ventana transparente, siempre encima, que no recibe clics, una por monitor.
- Motor "v2" del artboard: línea nítida + resplandor + esquinas más intensas + grano fino; ajuste automático en fondos claros.
- Por defecto aparece solo en el monitor activo (el de la ventana en primer plano al llegar el evento); opción "todos los monitores".

### Isla
- **Modo por defecto: Comando.** No ocupa espacio en reposo. Baja un aviso breve cuando pasa algo. `Ctrl+Alt+Espacio` abre una paleta con búsqueda: aprobar o denegar pendientes, modo foco, silenciar, abrir el tablero.
- **Modo alternativo: Orbe.** Un punto de luz sin caja. Un punto diminuto gira a su alrededor mientras trabaja; late cuando te necesita. Al pasar el mouse se estira en una línea de texto; al hacer clic muestra cuatro acciones; con un permiso se transforma en la tarjeta. Se puede arrastrar a la izquierda, centro o derecha.
- **Nunca los dos a la vez.** Se cambia en Ajustes ("Modo de la isla").
- Atajos globales `Ctrl+Alt+Y` (permitir) y `Ctrl+Alt+N` (denegar). Comandos delicados (`git push --force`, `rm -rf`, `migrate`) se aprueban manteniendo presionado 900 ms.
- Cuenta regresiva visible del tiempo de espera.

### Centro de mando: las dos piezas
- **Bolsillo:** panel que sube desde el ícono de la bandeja (380 × 540). Anillo de estado, cuatro botones rápidos (Foco, Silencio, Pulsos, Teclado), tarjetas por sesión con permiso dentro, pestañas Inicio, Ajustes y Hoy. Botón para abrir Mosaico.
- **Mosaico:** ventana completa con piezas reacomodables (Sesiones, Pendiente, Hoy, Acciones, Foco, Pulso, Teclas por sesión). Modo edición: arrastrar, quitar y agregar piezas. El orden se guarda.

### Modo foco
- Se activa en llamada (micrófono en uso), con app en pantalla completa o presentación, en un horario, o a mano.
- Regla por evento: Normal, Sin sonido, Nada, Después. Los "Después" se muestran juntos al salir del foco.

### Teclado
- Razer Chroma (SDK ya instalado en el equipo). F1, F2 y F3 se asignan a las sesiones y quedan encendidas mientras esa sesión espera.

### Bitácora
- Cada evento se guarda en `~/.claude/pulse/log.jsonl`. Sirve para la sección Hoy y para `/daily-report`.

### Sonidos
- Los cuatro `.wav` actuales (`start`, `permission`, `done`, `idle`) se reproducen desde la app, sincronizados con la luz. Error y subagente usan un tono corto. Créditos en `assets/sounds/CREDITS.md`.

### Ajustes de implementación
- **"Ir a la terminal" y "Responder «continúa»" no se implementan:** los hooks no dicen en qué ventana corre cada sesión ni hay API para escribirle a Claude. Se quitan de Comando y Bolsillo.
- **Pulso "en el monitor de la sesión" → "monitor activo":** el monitor de la ventana en primer plano al llegar el evento. Opción alterna: todos los monitores.
- **"No molestar" de Windows no se detecta** (no hay API pública). El foco se activa por llamada (micrófono en uso), pantalla completa/presentación, horario, o a mano.
- La forma "Desde la terminal" nace en una posición fija por sesión (`slot` 0, 1, 2… → 18 %, 50 %, 82 % del ancho).

### Descartado
- Celular (ntfy), radar flotante y barra de tareas como piezas sueltas. El radar vive dentro de Bolsillo y Mosaico.
- Islas Hilo, Cápsula, Pila y Ticker.
