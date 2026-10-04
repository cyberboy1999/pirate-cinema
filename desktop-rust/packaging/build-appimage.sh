#!/usr/bin/env bash
set -euo pipefail

version="${1:?version is required}"
binary="${2:?application binary is required}"
torrserver="${3:?TorrServer binary is required}"
output="${4:?output directory is required}"
linuxdeploy="${5:?linuxdeploy is required}"
appimagetool="${6:?appimagetool is required}"

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
  --desktop-file "$appdir/usr/share/applications/pirate-cinema.desktop" \
  --icon-file "$appdir/usr/share/icons/hicolor/256x256/apps/pirate-cinema.png" \
  --plugin gtk

# WebKitGTK helper paths and its GLib stack must come from the same
# distribution. Keep only Ubuntu's libxdo ABI compatibility library and use
# the host GTK/WebKit/MPV stack for everything else.
libxdo="$(find "$appdir/usr/lib" -type f -name 'libxdo.so.3*' -print -quit)"
test -n "$libxdo"
compat_lib="$(mktemp)"
cp -L "$libxdo" "$compat_lib"
find "$appdir/usr/lib" -depth -delete
mkdir -p "$appdir/usr/lib/compat"
install -m644 "$compat_lib" "$appdir/usr/lib/compat/libxdo.so.3"
rm -f "$compat_lib"
printf '%s\n' \
  '#!/bin/sh' \
  'HERE="$(dirname "$(readlink -f "$0")")"' \
  'export PATH="$HERE/usr/bin:$PATH"' \
  'export LD_LIBRARY_PATH="$HERE/usr/lib/compat${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"' \
  'exec "$HERE/usr/bin/pirate-cinema" "$@"' \
  > "$appdir/AppRun"
chmod 755 "$appdir/AppRun"

APPIMAGE_EXTRACT_AND_RUN=1 \
ARCH=x86_64 \
"$appimagetool" \
  "$appdir" \
  "$output/Pirate-Cinema-$version-x86_64.AppImage"
