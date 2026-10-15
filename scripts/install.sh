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

# Con sudo, HOME es /root: Pulse quedaría en la cuenta de root y target/ y app/dist serían de root.
if [ "$(id -u)" -eq 0 ]; then echo "No uses sudo: Pulse se instala en tu usuario. Corre scripts/install.sh sin sudo." >&2; exit 1; fi
# Paquetes del sistema (Ubuntu/Debian): los que falten se instalan con apt; solo ese paso pide tu contraseña.
pkgs=(libwebkit2gtk-4.1-0 libayatana-appindicator3-1 pulseaudio-utils)
[ "$build" = 0 ] || pkgs+=(build-essential curl pkg-config libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libasound2-dev libxdo-dev)
{ command -v node && command -v npm; } >/dev/null || pkgs+=(nodejs npm)
missing=()
for p in "${pkgs[@]}"; do dpkg-query -W -f='${Status}' "$p" 2>/dev/null | grep -q 'install ok installed' || missing+=("$p"); done
if [ ${#missing[@]} -gt 0 ]; then
  if ! command -v apt-get >/dev/null; then echo "Faltan estos paquetes (o sus equivalentes en tu distribución): ${missing[*]}" >&2; exit 1; fi
  echo "Faltan paquetes del sistema: ${missing[*]}"
  echo "Se instalan con apt (sudo te pide tu contraseña):"
  sudo apt-get install "${missing[@]}" || { sudo apt-get update && sudo apt-get install "${missing[@]}"; }
fi
node_major="$(node -p 'process.versions.node.split(".")[0]')"
if [ "$node_major" -lt 18 ]; then echo "Pulse necesita Node 18 o más nuevo y tienes $(node --version). Instala uno reciente con nvm o desde https://nodejs.org" >&2; exit 1; fi
# Rust: el de apt es muy viejo para Tauri; se instala y se actualiza con rustup.
if [ "$build" = 1 ]; then
  rustup_hint="curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
  if ! command -v cargo >/dev/null; then echo "Falta Rust. Instálalo con:  $rustup_hint  y vuelve a correr este script." >&2; exit 1; fi
  rust_minor="$(rustc --version | awk '{split($2, v, "."); print v[2]}')"
  if [ "${rust_minor:-0}" -lt 90 ]; then
    if command -v rustup >/dev/null; then echo "Pulse necesita Rust 1.90 o más nuevo y tienes $(rustc --version | awk '{print $2}'). Actualízalo con:  rustup update" >&2
    else echo "Pulse necesita Rust 1.90 o más nuevo y tienes $(rustc --version | awk '{print $2}') (el de apt). Quítalo (sudo apt remove cargo rustc) e instala Rust con:  $rustup_hint" >&2; fi
    exit 1
  fi
fi

if [ "$build" = 1 ]; then
  # la primera vez, y otra vez si package-lock.json cambió (después de un git pull)
  [ "$root/app/node_modules/.package-lock.json" -nt "$root/app/package-lock.json" ] || npm ci --prefix "$root/app"
  npm run build --prefix "$root/app"
  (cd "$root" && cargo build --release -p pulse-cli -p pulse-app --features pulse-app/custom-protocol)
fi
pkill -x pulse-app 2>/dev/null || true
mkdir -p "$dest"
install -m 755 "$root/target/release/pulse" "$root/target/release/pulse-app" "$dest/"
install -m 644 "$root/assets/icon.png" "$dest/icon.png"
# Sin esto GNOME muestra un engrane en el dock: reconoce la ventana por su clase (pulse-app) y busca su ícono aquí.
apps="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
mkdir -p "$apps"
cat > "$apps/pulse.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=Pulse
Comment=Lo que hace Claude Code, fuera de la terminal
Exec="$dest/pulse-app"
Icon=$dest/icon.png
StartupWMClass=pulse-app
Categories=Development;
DESKTOP
echo "Pulse instalado en $dest"
for d in "${dirs[@]}"; do node "$root/scripts/linux-setup.mjs" hooks-add "$d/settings.json" "$dest/pulse"; done
# Las cuentas con hooks quedan anotadas: uninstall.sh las limpia todas, aunque se le llame sin carpetas.
{ cat "$dest/accounts" 2>/dev/null || true; for d in "${dirs[@]}"; do realpath -m "$d"; done; } | awk 'NF && !s[$0]++' > "$dest/accounts.tmp" && mv "$dest/accounts.tmp" "$dest/accounts"
node "$root/scripts/linux-setup.mjs" keys-add "$dest/pulse"
echo "Listo. Pulse arranca solo con el próximo evento de Claude Code (las sesiones ya abiertas necesitan reiniciarse para leer los hooks)."
