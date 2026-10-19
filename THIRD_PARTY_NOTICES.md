# Third-party notices

Pirate Cinema includes and invokes third-party software. The notices below identify
the principal code and binaries used by the current Windows and Linux packages; each
component remains governed by its own license.

## librqbit

- Version: 8.1.1, linked into the Rust application.
- License: Apache-2.0.
- Project/source: https://github.com/ikatson/rqbit
- Cargo package: https://crates.io/crates/librqbit/8.1.1
- The application does not distribute or start a separate torrent-server executable.

## FFmpeg

- Version: 6.0 essentials build from gyan.dev, supplied by `ffmpeg-static@5.2.0`
- License for this build: GNU General Public License v3.0
- FFmpeg source revision: https://github.com/FFmpeg/FFmpeg/commit/ea3d24bbe3
- Binary package project: https://github.com/eugeneware/ffmpeg-static/tree/b6.0
- Build information: https://www.gyan.dev/ffmpeg/builds/
- Distributed file: `ffmpeg.exe`
- SHA-256: `e9fd5e711debab9d680955fc1e38a2c1160fd280b144476cc3f62bc43ef49db1`

Linux packages depend on the distribution-provided FFmpeg instead of
redistributing it.

## Microsoft Visual C++ Redistributable

- Version: 14.44.35211.0 (x64).
- Official Microsoft payload: `VC_redist.x64.exe`.
- License: Microsoft Software License Terms displayed and accepted by its installer.
- SHA-256: `cc0ff0eb1dc3f5188ae6300faef32bf5beeba4bdd6e8e445a9184072096b713b`.

## Development-only Electron and JavaScript packages

- Electron 43.4.0 is retained for the retired desktop implementation and
  development compatibility; it is not staged in the Rust release packages.
- JavaScript package names, versions and resolved sources are recorded in
  `package.json` and `pnpm-lock.yaml`. They are development/test dependencies
  unless explicitly identified elsewhere in this notice.

## Rust desktop packages

The production Rust desktop package versions and sources are recorded in
`desktop-rust/Cargo.lock`. Its principal libraries are Dioxus Desktop,
rusqlite/SQLite, ureq, image, rfd and tray-icon/muda. These packages are
distributed under their respective MIT, Apache-2.0 or SQLite public-domain
terms; their upstream license metadata is retained in the Cargo registry and
linked from `desktop-rust/Cargo.lock`.

The application embeds ArtPlayer 5.4.0 (MIT) and hls.js for its WebView2
playback controls. ArtPlayer source and license:
https://github.com/zhw2590582/ArtPlayer/tree/v5.4.0. The vendored
`desktop-rust/assets/artplayer.js` SHA-256 is
`f1db1fa3406aa6d41baa7c554749429a8e79f99d56a794bde23345eb9c07573f`.

## Ponytail

- Instruction package by Dietrich Gebert, commit
  `2ed6c52c9d7e5e56942508591085fd45dea277d3`
- License: MIT
- Included license: `.agents/skills/ponytail/LICENSE`
- Source: https://github.com/DietrichGebert/ponytail

## Metadata and artwork

- `public/pirate-cinema-logo.png` is derived from a user-provided third-party
  logo. It is not covered by Pirate Cinema's source-code license, and Pirate
  Cinema is not affiliated with or endorsed by The Pirate Bay.

Pirate Cinema does not claim ownership of the projects, trademarks or works
listed above.
