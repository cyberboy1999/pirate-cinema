#!/usr/bin/env bash
set -euo pipefail

archive="${1:?Pass the Pirate Cinema tar.gz archive}"
prefix="${XDG_DATA_HOME:-$HOME/.local/share}/pirate-cinema"
desktop="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
mkdir -p "$prefix" "$desktop"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
tar -xzf "$archive" -C "$tmp"
cp -a "$tmp/opt/pirate-cinema/." "$prefix/"
sed "s|/opt/pirate-cinema|$prefix|g" "$tmp/usr/share/applications/pirate-cinema.desktop" > "$desktop/pirate-cinema.desktop"
chmod 755 "$prefix/pirate-cinema" "$prefix/torrserver/TorrServer-linux-amd64"
command -v mpv >/dev/null 2>&1 || printf '%s\n' 'Warning: install mpv before playback.'
command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$desktop" || true
printf 'Pirate Cinema installed in %s\n' "$prefix"
