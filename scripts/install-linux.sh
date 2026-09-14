#!/bin/sh
set -eu

VERSION=0.5.1
TAG="v${VERSION}"
BASE="https://github.com/cyberboy1999/pirate-cinema/releases/download/${TAG}"

[ "$(uname -m)" = "x86_64" ] || { echo "Pirate Cinema supports Linux x86_64 only." >&2; exit 1; }
command -v sha256sum >/dev/null || { echo "sha256sum is required." >&2; exit 1; }

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT HUP INT TERM
download(){ if command -v curl >/dev/null; then curl -fL "$1" -o "$2"; elif command -v wget >/dev/null; then wget -O "$2" "$1"; else echo "Install curl or wget first." >&2; exit 1; fi; }

if command -v apt-get >/dev/null && command -v dpkg >/dev/null; then
  package="Pirate-Cinema-${VERSION}-linux-amd64.deb"
  manager=apt
elif command -v dnf >/dev/null || command -v yum >/dev/null; then
  package="Pirate-Cinema-${VERSION}-linux-x86_64.rpm"
  manager=rpm
else
  echo "Supported distributions: Debian/Ubuntu and Fedora/RHEL/openSUSE-compatible RPM systems." >&2
  exit 1
fi

download "$BASE/$package" "$tmp/$package"
download "$BASE/SHA256SUMS" "$tmp/SHA256SUMS"
(cd "$tmp" && grep "  $package\$" SHA256SUMS | sha256sum -c -)

sudo_cmd=""
[ "$(id -u)" -eq 0 ] || { command -v sudo >/dev/null || { echo "sudo is required for installation." >&2; exit 1; }; sudo_cmd=sudo; }
if [ "$manager" = apt ]; then $sudo_cmd apt-get install -y "$tmp/$package"; else command -v dnf >/dev/null && $sudo_cmd dnf install -y "$tmp/$package" || $sudo_cmd yum install -y "$tmp/$package"; fi

for dependency in mpv ffmpeg; do command -v "$dependency" >/dev/null || { echo "Missing dependency after installation: $dependency" >&2; exit 1; }; done
desktop_file="$(find /usr/share/applications -maxdepth 1 -iname '*pirate*cinema*.desktop' -print -quit)"
[ -n "$desktop_file" ] || { echo "Pirate Cinema was installed, but its desktop entry was not found." >&2; exit 1; }
mkdir -p "$HOME/.local/share/applications"
cp "$desktop_file" "$HOME/.local/share/applications/pirate-cinema.desktop"
desktop_dir="$(command -v xdg-user-dir >/dev/null && xdg-user-dir DESKTOP || printf '%s/Desktop' "$HOME")"
if [ -d "$desktop_dir" ]; then cp "$desktop_file" "$desktop_dir/Pirate Cinema.desktop"; chmod +x "$desktop_dir/Pirate Cinema.desktop"; command -v gio >/dev/null && gio set "$desktop_dir/Pirate Cinema.desktop" metadata::trusted true >/dev/null 2>&1 || true; fi
echo "Pirate Cinema $VERSION installed. Application menu shortcut created${desktop_dir:+; desktop shortcut: $desktop_dir}."
