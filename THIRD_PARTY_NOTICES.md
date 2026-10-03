# Third-party notices

Pirate Cinema 0.3.2 through 0.6.0 includes and invokes third-party software. The notices
below identify binaries distributed in the Windows and Linux installers; each
component remains governed by its own license.

## TorrServer

- Version: MatriX.144
- License: GNU General Public License v3.0
- Project and source: https://github.com/YouROK/TorrServer/tree/MatriX.144
- License text: https://github.com/YouROK/TorrServer/blob/MatriX.144/LICENSE
- Distributed file: `TorrServer-windows-amd64.exe`
- SHA-256: `3b68ba9409c009628bbf747a14c13f2da4254389264a0b3903e581e897492569`

The Linux packages contain the official `TorrServer-linux-amd64` binary from the
same release under GPL-3.0.

- Source: https://github.com/YouROK/TorrServer/tree/MatriX.144
- SHA-256: `e93e3c3b85932eed4c98b61785fabe6379b1ee10d91998729cbdea3384af5f9d`

## mpv

- Version: v0.40.0-330-gbe98b35c8
- Upstream license: GPL-2.0-or-later by default; LGPL-2.1-or-later applies only
  to builds made without GPL-only files. Treat this bundled build as GPL.
- Player source revision: https://github.com/mpv-player/mpv/commit/be98b35c8
- License information: https://github.com/mpv-player/mpv/blob/master/Copyright
- Windows build project: https://github.com/zhongfly/mpv-winbuild
- Distributed file: `mpv.exe`
- SHA-256: `5e3ad16b8195881b9d87efbb4103371b7fe89205789ce4235c8f51f67c419ac2`

The mpv binary statically includes separately licensed multimedia libraries.
The build project above documents those components and its reproducible build
configuration.

Linux packages depend on the distribution-provided MPV instead of redistributing it.

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

## Electron and JavaScript packages

- Electron 43.4.0 is licensed under MIT and includes Chromium third-party
  notices in `LICENSE.electron.txt` and `LICENSES.chromium.html` beside the
  installed executable.
- JavaScript package names, versions and resolved sources are recorded in
  `package.json` and `pnpm-lock.yaml`. Their own license terms apply.

## Rust desktop packages

The production Rust desktop package versions and sources are recorded in
`desktop-rust/Cargo.lock`. Its principal libraries are Dioxus Desktop,
rusqlite/SQLite, ureq, image, rfd and tray-icon/muda. These packages are
distributed under their respective MIT, Apache-2.0 or SQLite public-domain
terms; their upstream license metadata is retained in the Cargo registry and
linked from `desktop-rust/Cargo.lock`.

## Ponytail

- Instruction package by Dietrich Gebert, commit
  `2ed6c52c9d7e5e56942508591085fd45dea277d3`
- License: MIT
- Included license: `.agents/skills/ponytail/LICENSE`
- Source: https://github.com/DietrichGebert/ponytail

## Metadata and artwork

- Text excerpts attributed to Wikipedia are available under CC BY-SA 4.0 and
  are linked to their source articles in the application.
- `public/pirate-cinema-logo.png` is derived from a user-provided third-party
  logo. It is not covered by Pirate Cinema's source-code license, and Pirate
  Cinema is not affiliated with or endorsed by The Pirate Bay.

Pirate Cinema does not claim ownership of the projects, trademarks or works
listed above.
