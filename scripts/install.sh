#!/usr/bin/env bash
# Linux: compila Pulse, lo copia a ~/.local/share/pulse y registra los hooks de Claude Code y los atajos de GNOME.
# Uso: scripts/install.sh [--no-build] [carpeta de config de Claude ...]
#      Sin carpetas usa $CLAUDE_CONFIG_DIR o ~/.claude. Con varias cuentas: scripts/install.sh ~/.claude ~/.claude-work
set -euo pipefail
[ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"   # cargo recién instalado con rustup
root="$(cd "$(dirname "$0")/.." && pwd)"
dest="${XDG_DATA_HOME:-$HOME/.local/share}/pulse"
build=1; dirs=()
for a in "$@"; do case "$a" in --no-build) build=0 ;; *) dirs+=("$a") ;; esac; done
[ ${#dirs[@]} -gt 0 ] || dirs=("${CLAUDE_CONFIG_DIR:-$HOME/.claude}")

if [ "$build" = 1 ]; then
  npm run build --prefix "$root/app"
  (cd "$root" && cargo build --release -p pulse-cli -p pulse-app --features pulse-app/custom-protocol)
fi
pkill -x pulse-app 2>/dev/null || true
mkdir -p "$dest"
install -m 755 "$root/target/release/pulse" "$root/target/release/pulse-app" "$dest/"
echo "Pulse instalado en $dest"
for d in "${dirs[@]}"; do node "$root/scripts/linux-setup.mjs" hooks-add "$d/settings.json" "$dest/pulse"; done
node "$root/scripts/linux-setup.mjs" keys-add "$dest/pulse"
echo "Listo. Pulse arranca solo con el próximo evento de Claude Code (las sesiones ya abiertas necesitan reiniciarse para leer los hooks)."
