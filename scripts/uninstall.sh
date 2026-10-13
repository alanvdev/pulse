#!/usr/bin/env bash
# Linux: quita los hooks de Pulse (solo esas entradas, con respaldo), los atajos de GNOME y los binarios.
# Uso: scripts/uninstall.sh [carpeta de config de Claude ...]   (sin carpetas: $CLAUDE_CONFIG_DIR o ~/.claude)
# La configuración y la bitácora en ~/.claude/pulse se quedan.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
dest="${XDG_DATA_HOME:-$HOME/.local/share}/pulse"
dirs=("$@"); [ ${#dirs[@]} -gt 0 ] || dirs=("${CLAUDE_CONFIG_DIR:-$HOME/.claude}")
pkill -x pulse-app 2>/dev/null || true
for d in "${dirs[@]}"; do node "$root/scripts/linux-setup.mjs" hooks-remove "$d/settings.json"; done
node "$root/scripts/linux-setup.mjs" keys-remove
rm -f "$dest/pulse" "$dest/pulse-app"; rmdir "$dest" 2>/dev/null || true
echo "Pulse desinstalado."
