#requires -Version 7
param([switch]$NoBuild)
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$dest = Join-Path $env:LOCALAPPDATA 'Pulse'
$settingsPath = Join-Path $HOME '.claude\settings.json'
if (-not $NoBuild) {
  Push-Location (Join-Path $root 'app'); npm run build; Pop-Location
  Push-Location $root; cargo build --release -p pulse-cli -p pulse-app --features pulse-app/custom-protocol; Pop-Location
}
Get-Process pulse-app -ErrorAction SilentlyContinue | Stop-Process -Force
New-Item -ItemType Directory -Force $dest | Out-Null
Copy-Item (Join-Path $root 'target\release\pulse.exe'), (Join-Path $root 'target\release\pulse-app.exe') $dest -Force
$exe = (Join-Path $dest 'pulse.exe') -replace '\\', '/'
$stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
Copy-Item $settingsPath "$settingsPath.bak-pulse-$stamp"
$s = Get-Content $settingsPath -Raw | ConvertFrom-Json -AsHashtable
if (-not $s.ContainsKey('hooks')) { $s.hooks = @{} }
function Entry([string]$sub, [string]$matcher, [bool]$async, [int]$timeout) {
  $hook = @{ type = 'command'; command = "`"$exe`" $sub" }
  if ($async) { $hook.async = $true } else { $hook.timeout = $timeout }
  $e = @{ hooks = @($hook) }; if ($matcher) { $e.matcher = $matcher }; return $e
}
$events = [ordered]@{
  SessionStart = @('emit', $null, $true); SessionEnd = @('emit', $null, $true); UserPromptSubmit = @('emit', $null, $true)
  PreToolUse = @('emit', '*', $true); PermissionRequest = @('ask', $null, $false); Notification = @('emit', 'idle_prompt', $true)
  Stop = @('emit', $null, $true); PostToolUseFailure = @('emit', $null, $true); SubagentStart = @('emit', $null, $true); SubagentStop = @('emit', $null, $true)
}
foreach ($ev in $events.Keys) {
  $keep = @()
  if ($s.hooks.ContainsKey($ev)) { $keep = @($s.hooks[$ev] | Where-Object { -not ($_.hooks | Where-Object { $_.command -match 'sounds/play\.ps1|pulse\.exe' }) }) }
  $a = $events[$ev]; $s.hooks[$ev] = $keep + (Entry $a[0] $a[1] $a[2] 35)
}
$s | ConvertTo-Json -Depth 20 | Set-Content $settingsPath -Encoding utf8NoBOM
Write-Host "Pulse instalado en $dest. Respaldo: $settingsPath.bak-pulse-$stamp"
