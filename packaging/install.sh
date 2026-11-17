#!/usr/bin/env bash
# Installs Noodle for the current user (no root needed).
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
bin="$here/noodle"; [ -f "$bin" ] || bin="$here/../target/release/noodle"
install -Dm755 "$bin" "$HOME/.local/bin/noodle"
# icons in every size docks and menus ask for (Cairo-Dock, Plank, KDE, GNOME…), plus the SVG
icons="$here/icons"; [ -d "$icons" ] || icons="$here/../assets/icons"
for f in "$icons"/noodle-*.png; do
  s="$(basename "$f" .png)"; s="${s#noodle-}"
  install -Dm644 "$f" "$HOME/.local/share/icons/hicolor/${s}x${s}/apps/noodle.png"
done
svg="$here/noodle-icon.svg"; [ -f "$svg" ] || svg="$here/../assets/noodle-icon.svg"
if [ -f "$svg" ]; then install -Dm644 "$svg" "$HOME/.local/share/icons/hicolor/scalable/apps/noodle.svg"; fi
install -Dm644 "$icons/noodle-256.png" "$HOME/.local/share/pixmaps/noodle.png"
mkdir -p "$HOME/.local/share/applications"
# the menu entry points at the icon file directly too, so docks that don't look in the
# user's icon theme still find it
sed -e "s|^Icon=.*|Icon=$HOME/.local/share/icons/hicolor/256x256/apps/noodle.png|" -e "s|^Exec=noodle|Exec=$HOME/.local/bin/noodle|" "$here/noodle.desktop" > "$HOME/.local/share/applications/noodle.desktop"
chmod 644 "$HOME/.local/share/applications/noodle.desktop"
gtk-update-icon-cache -q -f -t "$HOME/.local/share/icons/hicolor" 2>/dev/null || true
update-desktop-database -q "$HOME/.local/share/applications" 2>/dev/null || true
mkdir -p "$HOME/.local/share/noodle/soundfonts" "$HOME/.local/share/noodle/songs"
echo "Installed → ~/.local/bin/noodle (and an app-menu entry)."
if ! ls /usr/share/soundfonts/*.sf2 /usr/share/sounds/sf2/*.sf2 "$HOME/.local/share/noodle/soundfonts/"*.sf2 >/dev/null 2>&1; then
  echo
  echo "No sound bank found yet. Install one:"
  echo "  Arch/CachyOS:   sudo pacman -S soundfont-fluid"
  echo "  Debian/Ubuntu:  sudo apt install fluid-soundfont-gm"
  echo "  Fedora:         sudo dnf install fluid-soundfont-gm"
  echo "…or drop any .sf2 into ~/.local/share/noodle/soundfonts"
fi
