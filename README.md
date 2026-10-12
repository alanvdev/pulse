<div align="center">

<img src="assets/icon.png" width="72" alt="">

# Pulse

**Lo que hace Claude Code, fuera de la terminal.**

Te avisa cuando una sesión termina o se detiene a pedir permiso,
y te deja responder desde donde estés.

Windows · Tauri 2 · Rust · TypeScript

</div>

---

## Por qué existe

Mientras Claude Code trabaja, uno se va a otra ventana: el navegador, el editor, una llamada. Y se pierden minutos en dos cosas:

- **No saber que ya terminó.** La sesión acabó hace rato y nadie se enteró.
- **Un permiso esperando.** Claude se detuvo a preguntar y la sesión quedó parada hasta que volviste a la terminal.

Pulse resuelve esas dos. Lo demás (varias sesiones a la vez, modo foco, el resumen del día) existe para apoyarlas.

## Cómo funciona

Pulse se alimenta de los propios **hooks de Claude Code**, así que sus señales son exactas: esta sesión pide permiso, esta terminó, esta falló. No adivina leyendo la terminal.

Cada evento llega por varios canales, cada uno con su nivel de intrusión, del más discreto al más visible:

| | |
|---|---|
| **Bordes** | Luz suave en el contorno de la pantalla, con una forma distinta por evento. |
| **Isla** | Arriba de la pantalla: avisos de una línea, tarjetas de permiso y una paleta. Dos modos: *Comando* y *Orbe*. |
| **Bandeja** | Un punto por sesión. Clic: *Bolsillo*, el panel rápido. Clic derecho: el menú. |
| **Mosaico** | El tablero completo: sesiones, el día, la bitácora y todos los ajustes. |

Además, sonidos por evento y, si tienes un teclado Razer, F1 a F3 encendidas por sesión.

## Lo que cumple

- **Responder sin ir a la terminal.** Permite o deniega desde la isla, la paleta o con atajos globales: `Ctrl+Alt+Y` permite, `Ctrl+Alt+N` deniega, `Ctrl+Alt+Espacio` abre la paleta.
- **Nunca decide por ti.** Si nadie responde en 30 s, Pulse no hace nada y Claude pregunta en la terminal, como siempre. Los comandos delicados (`git push --force`, `rm -rf`) se aprueban manteniendo presionado.
- **Contesta las preguntas de Claude.** Opciones, selección múltiple y respuesta libre, desde la isla.
- **Modo foco.** Se activa solo en llamadas, en pantalla completa o en tu horario. Cada evento decide qué hace mientras tanto: normal, sin sonido, nada, o guardarlo para después.
- **Invisible cuando no pasa nada.** Arranca con el primer hook y se cierra solo a los 5 minutos sin actividad.
- **Sin depender del color.** Cada estado tiene su forma o su icono, y todo respeta *reducir movimiento* de Windows.

## Instalar

Por ahora se instala desde el código. Necesitas Windows 11, [Rust](https://rustup.rs), [Node 20+](https://nodejs.org) y [PowerShell 7](https://aka.ms/powershell).

```powershell
git clone https://github.com/<usuario>/pulse
cd pulse
npm install --prefix app
pwsh scripts/install.ps1
```

El instalador compila, copia Pulse a `%LOCALAPPDATA%\Pulse` y registra los hooks en `~/.claude/settings.json`. Antes de tocarlo guarda un respaldo con la fecha, y solo reemplaza las entradas que llaman a Pulse.

Para quitarlo:

```powershell
pwsh scripts/uninstall.ps1
```

## Desarrollo

```powershell
npm run tauri dev --prefix app   # la app en desarrollo
pwsh scripts/demo.ps1            # simula eventos de Claude Code (sin argumentos lista los escenarios)
cargo test --workspace           # Rust: protocolo, CLI y núcleo
npm test --prefix app            # lógica de la interfaz
```

```
crates/pulse-protocol   tipos y traducción de los hooks
crates/pulse-cli        pulse.exe: lo llama cada hook y habla con la app por \\.\pipe\pulse
app/src-tauri           núcleo: estado, ventanas, bandeja, sonido, modo foco, teclado
app/ui                  bordes, isla, Bolsillo, menú de la bandeja y Mosaico
```

Qué es el producto y para quién: [`PRODUCT.md`](PRODUCT.md). Cómo se ve: [`DESIGN.md`](DESIGN.md).

## Tus datos

Todo se queda en tu equipo: la configuración en `~/.claude/pulse/config.json` y la bitácora del día en `~/.claude/pulse/log.jsonl`. Pulse no se conecta a internet.

---

<div align="center"><sub>Pulse no es un producto de Anthropic.</sub></div>
