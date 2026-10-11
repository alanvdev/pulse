# Pulse

Pulse muestra fuera de la terminal lo que está haciendo Claude Code: luz en los bordes de la pantalla, una isla arriba al centro, un panel en la bandeja y un tablero completo.

## Instalar y desinstalar

```powershell
pwsh scripts/install.ps1     # compila, copia a %LOCALAPPDATA%\Pulse y registra los hooks (con respaldo de settings.json)
pwsh scripts/uninstall.ps1   # restaura settings.json desde el respaldo más antiguo
```

## Pruebas

```powershell
cargo test --workspace
npm test --prefix app
```
