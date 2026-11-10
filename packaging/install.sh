#!/usr/bin/env bash
# Installs Noodle for the current user (no root needed).
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
bin="$here/noodle"; [ -f "$bin" ] || bin="$here/../target/release/noodle"
install -Dm755 "$bin" "$HOME/.local/bin/noodle"
install -Dm644 "$here/noodle.desktop" "$HOME/.local/share/applications/noodle.desktop"
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
