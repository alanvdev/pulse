<div align="center">

<img src="assets/icon.png" width="128" alt="Logo de Pulse">

<h1>Pulse</h1>

<p><b>Lo que hace Claude Code, fuera de la terminal.</b></p>

<p>Te avisa cuando una sesión termina o se detiene a pedir permiso,<br>y te deja responder desde donde estés.</p>

<p>
  <img src="https://img.shields.io/badge/Windows-11-111311?style=flat-square&logo=windows11&logoColor=eaedea" alt="Windows 11">
  <img src="https://img.shields.io/badge/Linux-GNOME-111311?style=flat-square&logo=linux&logoColor=eaedea" alt="Linux con GNOME">
  <img src="https://img.shields.io/badge/Tauri-2-111311?style=flat-square&logo=tauri&logoColor=eaedea" alt="Tauri 2">
  <img src="https://img.shields.io/badge/Rust-111311?style=flat-square&logo=rust&logoColor=eaedea" alt="Rust">
  <img src="https://img.shields.io/badge/React-18-111311?style=flat-square&logo=react&logoColor=eaedea" alt="React 18">
  <img src="https://img.shields.io/badge/para-Claude%20Code-111311?style=flat-square" alt="Para Claude Code">
  <a href="LICENSE"><img src="https://img.shields.io/badge/licencia-MIT-111311?style=flat-square" alt="Licencia MIT"></a>
</p>

<p>
  <a href="#por-qué-existe">Por qué existe</a> ·
  <a href="#cómo-funciona">Cómo funciona</a> ·
  <a href="#lo-que-cumple">Lo que cumple</a> ·
  <a href="#instalar">Instalar</a> ·
  <a href="#desarrollo">Desarrollo</a>
</p>

</div>

<br>

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
- **Nunca decide por ti.** Si nadie responde a tiempo (30 s de fábrica; de 10 a 120 s en Ajustes), Pulse no hace nada y Claude pregunta en la terminal, como siempre. Los comandos delicados (`git push --force`, `rm -rf`) se aprueban manteniendo presionado.
- **Contesta las preguntas de Claude.** Opciones, selección múltiple y respuesta libre, desde la isla.
- **La isla va donde quieras.** `Ctrl` + arrastrar la mueve (tarjeta, pastilla u Orbe) a cualquier lugar y a cualquier monitor; se queda ahí. `Ctrl` + doble clic la regresa arriba.
- **Modo foco.** Se activa solo en llamadas, en pantalla completa o en tu horario. Cada evento decide qué hace mientras tanto: normal, sin sonido, nada, o guardarlo para después.
- **Invisible cuando no pasa nada.** Arranca con el primer hook y se cierra solo a los 5 minutos sin actividad.
- **Sin depender del color.** Cada estado tiene su forma o su icono, y todo respeta *reducir movimiento* del sistema (en Windows: Accesibilidad › Efectos visuales › Efectos de animación; en GNOME: Configuración › Accesibilidad › Reducir animación).

## Instalar

Por ahora se instala desde el código. Necesitas Windows 11, [Rust](https://rustup.rs), [Node 20+](https://nodejs.org) y [PowerShell 7](https://aka.ms/powershell).

```powershell
git clone https://github.com/alanvdev/pulse
cd pulse
npm install --prefix app
pwsh scripts/install.ps1
```

El instalador compila, copia Pulse a `%LOCALAPPDATA%\Pulse` y registra los hooks en `~/.claude/settings.json`. Antes de tocarlo guarda un respaldo con la fecha, y solo reemplaza las entradas que llaman a Pulse.

Para quitarlo:

```powershell
pwsh scripts/uninstall.ps1
```

### Linux (GNOME)

Probado en Ubuntu 25.10 con GNOME en Wayland. Necesitas [Rust](https://rustup.rs), [Node 20+](https://nodejs.org), el compilador de C (`build-essential`), las librerías de Tauri y `pactl` (`pulseaudio-utils`, para detectar llamadas en el modo foco):

```bash
sudo apt install build-essential curl pulseaudio-utils libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libasound2-dev libxdo-dev pkg-config
git clone https://github.com/alanvdev/pulse
cd pulse
npm install --prefix app
scripts/install.sh                          # hooks en $CLAUDE_CONFIG_DIR o ~/.claude
scripts/install.sh ~/.claude ~/.claude-work # o en varias cuentas de Claude Code a la vez
```

El instalador compila, copia Pulse a `~/.local/share/pulse`, registra los hooks en cada `settings.json` (con un respaldo fechado, y solo toca los hooks que llaman a Pulse: los tuyos se quedan aunque compartan grupo con ellos) y da de alta los atajos `Ctrl+Alt+Y`, `Ctrl+Alt+N` y `Ctrl+Alt+Espacio` como atajos de GNOME.

Para quitarlo: `scripts/uninstall.sh`. Sin argumentos limpia todas las cuentas donde el instalador dejó hooks (y cualquier `~/.claude*/settings.json` que todavía llame a Pulse); también acepta carpetas extra. Quita solo los hooks de Pulse y los atajos, y cierra la app al final; tu configuración en `~/.claude/pulse` se queda. Las sesiones de Claude Code que estén abiertas siguen con los hooks que leyeron al empezar: reinícialas.

Diferencias con Windows:

- **Corre por XWayland.** Wayland no deja que una app se ponga encima de las demás ni elija dónde va su ventana, y los bordes y la isla lo necesitan. Pulse lo activa solo.
- **La bandeja abre un menú.** El área de notificación de GNOME no le pasa los clics a la app: el ícono abre un menú con Bolsillo, el menú rápido, la paleta y Mosaico.
- **Atajos sin "mantener".** Los atajos de GNOME son un golpe de tecla, así que un comando delicado se aprueba manteniendo el botón de la isla (`Ctrl+Alt+Y` sirve para los demás).
- **Modo foco.** Detecta llamadas (cualquier app grabando del micrófono) y tu horario; la pantalla completa no, porque Wayland no deja ver las ventanas de otras apps, así que esa opción no aparece en Ajustes.
- **Sin Razer Chroma.** Su SDK solo existe en Windows; sus opciones no aparecen.
- **Pulsos en todos los monitores.** Wayland no deja saber en qué ventana ni en qué monitor estás, así que no hay "monitor activo" y la opción *Dónde* no aparece en Ajustes.
- **La isla se mueve igual.** `Ctrl` + arrastrar la lleva a cualquier lugar y a cualquier monitor; `Ctrl` + doble clic la regresa arriba.
- **En pantalla completa GNOME baja la isla.** Si llega un permiso o una pregunta, la tarjeta queda tapada: suena y se encienden los bordes. Un permiso normal se contesta con `Ctrl+Alt+Y` / `Ctrl+Alt+N`; uno delicado (el que pide mantener el botón), con `Ctrl+Alt+N`, en la terminal o saliendo de pantalla completa; una pregunta, en la terminal o saliendo de pantalla completa.

## Desarrollo

Solo puede correr un Pulse a la vez, porque uno solo puede ocupar `\\.\pipe\pulse` (en Linux, `pulse.sock`): antes de `npm run tauri dev` cierra el instalado (bandeja › Salir, o `pkill -x pulse-app` en Linux), o la versión de desarrollo se cierra sola al arrancar. Al terminar cierra también la de desarrollo, porque mientras viva se queda con los hooks y los atajos.

```powershell
npm run tauri dev --prefix app   # la app en desarrollo
pwsh scripts/demo.ps1            # simula eventos de Claude Code (sin argumentos lista los escenarios); en Linux: scripts/demo.sh
cargo test --workspace           # Rust: protocolo, CLI y núcleo
npm test --prefix app            # lógica de la interfaz
```

```
crates/pulse-protocol   tipos y traducción de los hooks
crates/pulse-cli        pulse.exe: lo llama cada hook y habla con la app por \\.\pipe\pulse (en Linux, $XDG_RUNTIME_DIR/pulse.sock)
app/src-tauri           núcleo: estado, ventanas, bandeja, sonido, modo foco, teclado
app/ui                  bordes, isla, Bolsillo, menú de la bandeja y Mosaico
```

Qué es el producto y para quién: [`PRODUCT.md`](PRODUCT.md). Cómo se ve: [`DESIGN.md`](DESIGN.md).

## Tus datos

Todo se queda en tu equipo: la configuración en `~/.claude/pulse/config.json`, la bitácora de los últimos 7 días en `~/.claude/pulse/log.jsonl` y, si la app no arranca, el motivo en `~/.claude/pulse/app.log`. Pulse no se conecta a internet.

---

<div align="center"><sub>Licencia <a href="LICENSE">MIT</a> · Pulse no es un producto de Anthropic.</sub></div>
