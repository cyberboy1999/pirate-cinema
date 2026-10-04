#!/usr/bin/env bash
set -euo pipefail

version="${1:?version is required}"
binary="${2:?application binary is required}"
torrserver="${3:?TorrServer binary is required}"
output="${4:?output directory is required}"
linuxdeploy="${5:?linuxdeploy is required}"

root="$(cd "$(dirname "$0")/.." && pwd)"
appdir="$(mktemp -d)"
trap 'rm -rf "$appdir"' EXIT

mkdir -p "$appdir/usr/bin/torrserver" "$appdir/usr/share/applications" "$appdir/usr/share/icons/hicolor/256x256/apps" "$output"
install -m755 "$binary" "$appdir/usr/bin/pirate-cinema"
install -m755 "$torrserver" "$appdir/usr/bin/torrserver/TorrServer-linux-amd64"
sed 's|Exec=/opt/pirate-cinema/pirate-cinema %U|Exec=pirate-cinema %U|' "$root/packaging/pirate-cinema.desktop" > "$appdir/usr/share/applications/pirate-cinema.desktop"
install -m644 "$root/../public/favicon.png" "$appdir/usr/share/icons/hicolor/256x256/apps/pirate-cinema.png"

APPIMAGE_EXTRACT_AND_RUN=1 \
NO_STRIP=1 \
"$linuxdeploy" \
  --appdir "$appdir" \
  --executable "$appdir/usr/bin/pirate-cinema" \
  --executable /usr/bin/mpv \
  --desktop-file "$appdir/usr/share/applications/pirate-cinema.desktop" \
  --icon-file "$appdir/usr/share/icons/hicolor/256x256/apps/pirate-cinema.png" \
  --plugin gtk

# WebKitGTK helper paths are compiled into the library and differ between
# distributions. Use the host WebKitGTK stack so its helpers always match;
# the package still bundles the rest of the application runtime.
find "$appdir/usr/lib" \( -type f -o -type l \) \( \
  -name 'libwebkit2gtk-4.1.so*' -o \
  -name 'libjavascriptcoregtk-4.1.so*' \
\) -delete

APPIMAGE_EXTRACT_AND_RUN=1 \
NO_STRIP=1 \
OUTPUT="$output/Pirate-Cinema-$version-x86_64.AppImage" \
"$linuxdeploy" \
  --appdir "$appdir" \
  --output appimage
