#requires -Version 7
# Simula eventos de Claude Code para probar Pulse sin esperar a que pasen de verdad.
# Uso: pwsh scripts/demo.ps1 <escenario>      (sin escenario muestra la lista)
param([string]$Escenario = '')
$ErrorActionPreference = 'Stop'
$pulse = Join-Path $env:LOCALAPPDATA 'Pulse\pulse.exe'
if (-not (Test-Path $pulse)) { throw "No encuentro $pulse. Corre primero: pwsh scripts/install.ps1" }
# Siempre la misma sesión: así `fin` cierra la que dejaron los escenarios anteriores, aunque los hayas corrido en otra ventana.
$sesion = 'pulse-demo'

function Hook([string]$modo, [hashtable]$datos) {
  $base = @{ session_id = $sesion; cwd = 'D:\demo\pulse-demo' }
  foreach ($k in $datos.Keys) { $base[$k] = $datos[$k] }
  $json = $base | ConvertTo-Json -Depth 10 -Compress
  $salida = $json | & $pulse $modo
  if ($modo -eq 'ask') { if ($salida) { "Claude recibiría: $salida" } else { 'Sin decisión: Claude preguntaría en la terminal.' } }
}
$pregunta = @{ question = '¿Con qué framework sigo?'; header = 'Framework'; multiSelect = $false; options = @(@{ label = 'React'; description = 'Componentes' }, @{ label = 'Vue'; description = 'Progresivo' }) }
$multiple = @{ question = '¿Qué partes reviso?'; header = 'Partes'; multiSelect = $true; options = @(@{ label = 'Isla' }, @{ label = 'Panel' }, @{ label = 'Tablero' }) }

$escenarios = [ordered]@{
  inicio     = @('Claude empieza a trabajar (respira blanco + sonido)', { Hook emit @{ hook_event_name = 'UserPromptSubmit'; prompt = 'Arregla el header en móvil' } })
  herramienta = @('Claude usa una herramienta (solo actividad, sin pulso)', { Hook emit @{ hook_event_name = 'PreToolUse'; tool_name = 'Edit'; tool_input = @{ file_path = 'src/Header.tsx' } } })
  permiso    = @('Pide permiso para un comando (espera tu Ctrl+Alt+Y / N o clic, 30 s)', { Hook ask @{ hook_event_name = 'PermissionRequest'; tool_name = 'Bash'; tool_input = @{ command = 'npm run build'; description = 'Compilar el proyecto' } } })
  delicado   = @('Comando delicado: hay que MANTENER Ctrl+Alt+Y (o el botón)', { Hook ask @{ hook_event_name = 'PermissionRequest'; tool_name = 'Bash'; tool_input = @{ command = 'git push --force origin main'; description = 'Forzar el push a main' } } })
  otra       = @('Permiso de otra herramienta (Artifact)', { Hook ask @{ hook_event_name = 'PermissionRequest'; tool_name = 'Artifact'; tool_input = @{ action = 'delete'; url = 'https://claude.ai/artifact/EJEMPLO' } } })
  pregunta   = @('Claude te pregunta (una opción: con un clic basta, 60 s)', { Hook ask @{ hook_event_name = 'PreToolUse'; tool_name = 'AskUserQuestion'; tool_input = @{ questions = @($pregunta) } } })
  preguntas  = @('Dos preguntas, una de varias opciones; prueba "Otra…"', { Hook ask @{ hook_event_name = 'PreToolUse'; tool_name = 'AskUserQuestion'; tool_input = @{ questions = @($multiple, $pregunta) } } })
  largas     = @('Dos preguntas largas con descripciones (una a la vez, Atrás / Siguiente / Enviar)', { Hook ask @{ hook_event_name = 'PreToolUse'; tool_name = 'AskUserQuestion'; tool_input = @{ questions = @(
      @{ question = '¿Cómo se ven los colores de los eventos en la versión final?'; header = 'Eventos'; multiSelect = $false; options = @(@{ label = 'Solo permiso en color'; description = 'Ámbar solo cuando algo te necesita; terminó y error en gris con su icono.' }, @{ label = 'Apagados'; description = 'Ámbar, verde y rojo desaturados, cada uno con su icono.' }, @{ label = 'Acento'; description = "La salvia marca 'te necesita'; menos colores en total." }) },
      @{ question = '¿Qué letra?'; header = 'Letra'; multiSelect = $false; options = @(@{ label = 'Geist'; description = 'La actual. Neutra y técnica, al estilo Vercel.' }, @{ label = 'Hanken Grotesk'; description = 'Más cálida y redonda.' }, @{ label = 'Zen Kaku Gothic'; description = 'Geométrica, con aire japonés.' }, @{ label = 'Inter'; description = 'La de siempre.' }) }) } } })
  plan       = @('Claude terminó un plan (aviso: revísalo en la terminal)', { Hook ask @{ hook_event_name = 'PermissionRequest'; tool_name = 'ExitPlanMode'; tool_input = @{} } })
  termino    = @('Claude terminó (ola verde + sonido + aviso en la isla)', { Hook emit @{ hook_event_name = 'Stop' } })
  error      = @('Falló una herramienta (destello rojo de costado)', { Hook emit @{ hook_event_name = 'PostToolUseFailure'; error = 'tsx no encontrado' } })
  espera     = @('Claude te espera (respira gris, brillo que se queda)', { Hook emit @{ hook_event_name = 'Notification'; notification_type = 'idle_prompt' } })
  subagente  = @('Un subagente terminó (destello azul, sin sonido)', { Hook emit @{ hook_event_name = 'SubagentStop'; agent_type = 'Explore' } })
  terminal   = @('Respondes en la terminal: la isla deja de esperar sola', {
      $job = Start-Job -ScriptBlock { param($p, $s) (@{ session_id = $s; cwd = 'D:\demo\pulse-demo'; hook_event_name = 'PermissionRequest'; tool_name = 'Bash'; tool_input = @{ command = 'npm test' } } | ConvertTo-Json -Compress) | & $p ask } -ArgumentList $pulse, $sesion
      Start-Sleep 3; 'Simulo que Claude siguió (ya respondiste en la terminal)…'; Hook emit @{ hook_event_name = 'Stop' }; Remove-Job $job -Force })
  todo       = @('Todos los escenarios seguidos (los permisos y preguntas esperan tu respuesta o se vencen solos)', {
      foreach ($n in @($escenarios.Keys | Where-Object { $_ -notin 'todo', 'fin' }) + 'fin') { "→ $n · $($escenarios[$n][0])"; & $escenarios[$n][1]; Start-Sleep 3 } })
  fin        = @('Cierra la sesión de prueba', { Hook emit @{ hook_event_name = 'SessionEnd' } })
}

if (-not $escenarios.Contains($Escenario)) {
  'Escenarios (pwsh scripts/demo.ps1 <nombre>):'; $escenarios.GetEnumerator() | ForEach-Object { '  {0,-12} {1}' -f $_.Key, $_.Value[0] }; return
}
"▶ $($escenarios[$Escenario][0])"
& $escenarios[$Escenario][1]
if ($Escenario -notin 'todo', 'fin') { 'Cuando termines de probar: pwsh scripts/demo.ps1 fin (quita la sesión de prueba de Pulse)' }
