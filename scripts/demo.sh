#!/usr/bin/env bash
# Linux: simula eventos de Claude Code para probar Pulse sin esperar a que pasen de verdad.
# Uso: scripts/demo.sh <escenario>      (sin escenario muestra la lista)
set -uo pipefail
pulse="${XDG_DATA_HOME:-$HOME/.local/share}/pulse/pulse"
[ -x "$pulse" ] || { echo "No encuentro $pulse. Corre primero: scripts/install.sh"; exit 1; }
# Siempre la misma sesión: así `fin` cierra la que dejaron los escenarios anteriores.
base='"session_id":"pulse-demo","cwd":"/home/demo/pulse-demo"'

hook() {   # hook <emit|ask> '<campos JSON sin llaves>'
  local salida; salida=$(printf '{%s,%s}' "$base" "$2" | "$pulse" "$1")
  if [ "$1" = ask ]; then if [ -n "$salida" ]; then echo "Claude recibiría: $salida"; else echo 'Sin decisión: Claude preguntaría en la terminal.'; fi; fi
}
pregunta='{"question":"¿Con qué framework sigo?","header":"Framework","multiSelect":false,"options":[{"label":"React","description":"Componentes"},{"label":"Vue","description":"Progresivo"}]}'
multiple='{"question":"¿Qué partes reviso?","header":"Partes","multiSelect":true,"options":[{"label":"Isla"},{"label":"Panel"},{"label":"Tablero"}]}'
largas1='{"question":"¿Cómo se ven los colores de los eventos en la versión final?","header":"Eventos","multiSelect":false,"options":[{"label":"Solo permiso en color","description":"Ámbar solo cuando algo te necesita; terminó y error en gris con su icono."},{"label":"Apagados","description":"Ámbar, verde y rojo desaturados, cada uno con su icono."},{"label":"Acento","description":"La salvia marca '\''te necesita'\''; menos colores en total."}]}'
largas2='{"question":"¿Qué letra?","header":"Letra","multiSelect":false,"options":[{"label":"Geist","description":"La actual. Neutra y técnica, al estilo Vercel."},{"label":"Hanken Grotesk","description":"Más cálida y redonda."},{"label":"Zen Kaku Gothic","description":"Geométrica, con aire japonés."},{"label":"Inter","description":"La de siempre."}]}'

nombres=(inicio herramienta permiso delicado otra pregunta preguntas largas plan termino error espera subagente terminal todo fin)
declare -A desc=(
  [inicio]='Claude empieza a trabajar (respira blanco + sonido)'
  [herramienta]='Claude usa una herramienta (solo actividad, sin pulso)'
  [permiso]='Pide permiso para un comando (espera tu Ctrl+Alt+Y / N o clic; 30 s de fábrica, se cambia en Ajustes)'
  [delicado]='Comando delicado: hay que MANTENER el botón de la isla'
  [otra]='Permiso de otra herramienta (Artifact)'
  [pregunta]='Claude te pregunta (una opción: con un clic basta, 60 s)'
  [preguntas]='Dos preguntas, una de varias opciones; prueba "Otra…"'
  [largas]='Dos preguntas largas con descripciones (una a la vez, Atrás / Siguiente / Enviar)'
  [plan]='Claude terminó un plan (aviso: revísalo en la terminal)'
  [termino]='Claude terminó (ola verde + sonido + aviso en la isla)'
  [error]='Falló una herramienta (destello rojo de costado)'
  [espera]='Claude te espera (respira gris, brillo que se queda)'
  [subagente]='Un subagente terminó (destello azul, sin sonido)'
  [terminal]='Respondes en la terminal: la isla deja de esperar sola'
  [todo]='Todos los escenarios seguidos (los permisos y preguntas esperan tu respuesta o se vencen solos)'
  [fin]='Cierra la sesión de prueba'
)

correr() {
  case "$1" in
    inicio) hook emit '"hook_event_name":"UserPromptSubmit","prompt":"Arregla el header en móvil"' ;;
    herramienta) hook emit '"hook_event_name":"PreToolUse","tool_name":"Edit","tool_input":{"file_path":"src/Header.tsx"}' ;;
    permiso) hook ask '"hook_event_name":"PermissionRequest","tool_name":"Bash","tool_input":{"command":"npm run build","description":"Compilar el proyecto"}' ;;
    delicado) hook ask '"hook_event_name":"PermissionRequest","tool_name":"Bash","tool_input":{"command":"git push --force origin main","description":"Forzar el push a main"}' ;;
    otra) hook ask '"hook_event_name":"PermissionRequest","tool_name":"Artifact","tool_input":{"action":"delete","url":"https://claude.ai/artifact/EJEMPLO"}' ;;
    pregunta) hook ask "\"hook_event_name\":\"PreToolUse\",\"tool_name\":\"AskUserQuestion\",\"tool_input\":{\"questions\":[$pregunta]}" ;;
    preguntas) hook ask "\"hook_event_name\":\"PreToolUse\",\"tool_name\":\"AskUserQuestion\",\"tool_input\":{\"questions\":[$multiple,$pregunta]}" ;;
    largas) hook ask "\"hook_event_name\":\"PreToolUse\",\"tool_name\":\"AskUserQuestion\",\"tool_input\":{\"questions\":[$largas1,$largas2]}" ;;
    plan) hook ask '"hook_event_name":"PermissionRequest","tool_name":"ExitPlanMode","tool_input":{}' ;;
    termino) hook emit '"hook_event_name":"Stop"' ;;
    error) hook emit '"hook_event_name":"PostToolUseFailure","error":"tsx no encontrado"' ;;
    espera) hook emit '"hook_event_name":"Notification","notification_type":"idle_prompt"' ;;
    subagente) hook emit '"hook_event_name":"SubagentStop","agent_type":"Explore"' ;;
    terminal)
      # Un solo comando en segundo plano, así $! es el propio `pulse ask` y se cierra solo ese: nada de pkill, que
      # también mataría los `pulse ask` de las sesiones reales que estén esperando en la isla.
      "$pulse" ask <<<"$(printf '{%s,%s}' "$base" '"hook_event_name":"PermissionRequest","tool_name":"Bash","tool_input":{"command":"npm test"}')" >/dev/null & local pid=$!
      sleep 3; echo 'Simulo que Claude siguió (ya respondiste en la terminal)…'; hook emit '"hook_event_name":"Stop"'; kill "$pid" 2>/dev/null; wait "$pid" 2>/dev/null ;;
    todo) for n in "${nombres[@]}"; do case "$n" in todo|fin) continue ;; esac; echo "→ $n · ${desc[$n]}"; correr "$n"; sleep 3; done; echo "→ fin · ${desc[fin]}"; correr fin ;;
    fin) hook emit '"hook_event_name":"SessionEnd"' ;;
  esac
}

e="${1:-}"
if [ -z "$e" ] || [ -z "${desc[$e]+x}" ]; then
  echo 'Escenarios (scripts/demo.sh <nombre>):'; for n in "${nombres[@]}"; do printf '  %-12s %s\n' "$n" "${desc[$n]}"; done; exit 0
fi
echo "▶ ${desc[$e]}"
correr "$e"
case "$e" in todo|fin) ;; *) echo 'Cuando termines de probar: scripts/demo.sh fin (quita la sesión de prueba de Pulse)' ;; esac
