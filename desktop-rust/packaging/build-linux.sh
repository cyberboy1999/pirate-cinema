#!/usr/bin/env bash
set -euo pipefail

version="${1:?version is required}"
binary="${2:-target/release/pirate-cinema}"
torrserver="${3:-vendor/torrserver/TorrServer-linux-amd64}"
output="${4:-../local-builds}"
root="$(cd "$(dirname "$0")/.." && pwd)"
stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT

test -x "$binary"
test -f "$torrserver"
mkdir -p "$output" "$stage/opt/pirate-cinema/torrserver" "$stage/usr/share/applications" "$stage/usr/share/icons/hicolor/256x256/apps"
install -m755 "$binary" "$stage/opt/pirate-cinema/pirate-cinema"
install -m755 "$torrserver" "$stage/opt/pirate-cinema/torrserver/TorrServer-linux-amd64"
install -m644 "$root/packaging/pirate-cinema.desktop" "$stage/usr/share/applications/pirate-cinema.desktop"
install -m644 "$root/../public/favicon.png" "$stage/usr/share/icons/hicolor/256x256/apps/pirate-cinema.png"

mkdir -p "$stage/DEBIAN"
cat > "$stage/DEBIAN/control" <<EOF
Package: pirate-cinema
Version: $version
Section: video
Priority: optional
Architecture: amd64
Depends: mpv, libgtk-3-0, libwebkit2gtk-4.1-0, libayatana-appindicator3-1, libxdo3
Maintainer: Pirate Cinema contributors
Description: Local TorrServer media library written in Rust
EOF
dpkg-deb --build --root-owner-group "$stage" "$output/Pirate-Cinema-$version-linux-amd64.deb"

archive="$output/Pirate-Cinema-$version-linux-x86_64.tar.gz"
tar -C "$stage" -czf "$archive" opt usr
sed "s/@VERSION@/$version/g" "$root/packaging/PKGBUILD.in" > "$output/PKGBUILD"
cp "$root/packaging/install-linux.sh" "$output/Pirate-Cinema-$version-install.sh"
chmod 755 "$output/Pirate-Cinema-$version-install.sh"

if command -v rpmbuild >/dev/null 2>&1; then
  top="$stage/rpmbuild"
  mkdir -p "$top"/{BUILD,BUILDROOT,RPMS,SOURCES,SPECS,SRPMS}
  cp "$archive" "$top/SOURCES/pirate-cinema.tar.gz"
  sed "s/@VERSION@/$version/g" "$root/packaging/pirate-cinema.spec.in" > "$top/SPECS/pirate-cinema.spec"
  rpmbuild --define "_topdir $top" -bb "$top/SPECS/pirate-cinema.spec"
  find "$top/RPMS" -name '*.rpm' -exec cp {} "$output/Pirate-Cinema-$version-linux-x86_64.rpm" \;
fi
