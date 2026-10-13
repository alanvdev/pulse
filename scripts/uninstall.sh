#!/usr/bin/env bash
# Linux: quita los hooks de Pulse (solo esas entradas, con respaldo), los atajos de GNOME y los binarios.
# Uso: scripts/uninstall.sh [carpeta de config de Claude ...]
#      Limpia las carpetas que le pases y, además, todas las cuentas donde install.sh dejó hooks (anotadas en
#      ~/.local/share/pulse/accounts) y cualquier ~/.claude*/settings.json que todavía llame a Pulse: el binario se borra
#      para todas, así que ninguna puede quedarse con hooks apuntando a él.
# La configuración y la bitácora en ~/.claude/pulse se quedan.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
dest="${XDG_DATA_HOME:-$HOME/.local/share}/pulse"
dirs=("$@" "${CLAUDE_CONFIG_DIR:-$HOME/.claude}")
[ -f "$dest/accounts" ] && mapfile -t -O "${#dirs[@]}" dirs < "$dest/accounts"
for f in "$HOME"/.claude*/settings.json; do [ -f "$f" ] && grep -qF "$dest/pulse\\\"" "$f" && dirs+=("$(dirname "$f")"); done
mapfile -t dirs < <(for d in "${dirs[@]}"; do [ -n "$d" ] && realpath -m "$d"; done | awk '!s[$0]++')
# Primero los hooks y después la app: un hook que llegue mientras tanto ya no encuentra a quién relanzar.
for d in "${dirs[@]}"; do node "$root/scripts/linux-setup.mjs" hooks-remove "$d/settings.json" || echo "  no se pudo leer $d/settings.json: quita a mano los hooks de Pulse" >&2; done
node "$root/scripts/linux-setup.mjs" keys-remove
rm -f "$dest/pulse" "$dest/pulse-app" "$dest/icon.png" "$dest/accounts" "${XDG_DATA_HOME:-$HOME/.local/share}/applications/pulse.desktop"; rmdir "$dest" 2>/dev/null || true
pkill -x pulse-app 2>/dev/null || true
echo "Pulse desinstalado. Las sesiones de Claude Code ya abiertas guardan los hooks que leyeron al empezar: reinícialas, o marcarán error de hook cada vez que llamen a Pulse."
