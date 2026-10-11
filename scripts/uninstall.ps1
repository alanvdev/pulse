#requires -Version 7
$ErrorActionPreference = 'Stop'
$settingsPath = Join-Path $HOME '.claude\settings.json'
$backup = Get-ChildItem "$settingsPath.bak-pulse-*" | Sort-Object Name | Select-Object -First 1   # el más antiguo = antes de Pulse
if (-not $backup) { throw 'No hay respaldo de Pulse.' }
Get-Process pulse-app -ErrorAction SilentlyContinue | Stop-Process -Force
Copy-Item $backup.FullName $settingsPath -Force
Write-Host "settings.json restaurado desde $($backup.Name)"
