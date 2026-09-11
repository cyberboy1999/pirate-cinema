# Third-party notices

Pirate Cinema 0.3.2 through 0.3.5 include and invoke third-party software. The notices
below identify the binaries distributed in the Windows installer; each
component remains governed by its own license.

## TorrServer

- Version: MatriX.135
- License: GNU General Public License v3.0
- Project and source: https://github.com/YouROK/TorrServer/tree/MatriX.135
- License text: https://github.com/YouROK/TorrServer/blob/MatriX.135/LICENSE
- Distributed file: `TorrServer-windows-amd64.exe`
- SHA-256: `f05c13f218286866473cb4e83b081eecd4cde8dcc8faf5ede321bfeaa444b044`

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

## FFmpeg

- Version: 6.0 essentials build from gyan.dev, supplied by `ffmpeg-static@5.2.0`
- License for this build: GNU General Public License v3.0
- FFmpeg source revision: https://github.com/FFmpeg/FFmpeg/commit/ea3d24bbe3
- Binary package project: https://github.com/eugeneware/ffmpeg-static/tree/b6.0
- Build information: https://www.gyan.dev/ffmpeg/builds/
- Distributed file: `ffmpeg.exe`
- SHA-256: `e9fd5e711debab9d680955fc1e38a2c1160fd280b144476cc3f62bc43ef49db1`

## Electron and JavaScript packages

- Electron 43.4.0 is licensed under MIT and includes Chromium third-party
  notices in `LICENSE.electron.txt` and `LICENSES.chromium.html` beside the
  installed executable.
- JavaScript package names, versions and resolved sources are recorded in
  `package.json` and `pnpm-lock.yaml`. Their own license terms apply.

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
