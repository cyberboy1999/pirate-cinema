# Pirate Cinema — local media library

Локальная медиатека с поиском раздач, постерами и стримингом через TorrServer.

## Run locally

```powershell
pnpm install
pnpm dev:local
```

Open `http://localhost:3000`. The integration API runs on `127.0.0.1:3001` and connects to TorrServer at `http://127.0.0.1:8090` by default.

Metadata is stored in `media.db` under `%APPDATA%\TorrServerDesktop`. When that location is unavailable, development falls back to `local-data/` in the project. Posters are cached beside the database.

Third-party components and their licenses are listed in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).

The Windows release provides two installation modes:

- **Web Setup** downloads the complete versioned application package during
  installation and installs MPV, TorrServer and FFmpeg inside Pirate Cinema.
- **Offline Setup** already contains the same runtime components and works
  without a network connection.

Use Offline Setup where GitHub downloads are blocked or require a VPN. Web
Setup requires access to the release asset for the entire installation.

Nothing is compiled on the user's computer. Both installers deploy the
prebuilt, version-pinned application package.

## Linux

Release 0.4.1 provides x86_64 packages for Debian/Ubuntu (`.deb`) and
RPM-based distributions (`.rpm`). MPV and FFmpeg are installed through the
system package manager; TorrServer is bundled and runs only for Pirate Cinema.

The accompanying `install-linux.sh` detects the package family, verifies the
download against `SHA256SUMS`, installs dependencies and creates a standard
freedesktop launcher in the application menu. If the user's Desktop directory
exists, it also creates a trusted desktop shortcut compatible with KDE, GNOME
and XFCE.

```sh
chmod +x install-linux.sh
./install-linux.sh
```

Arch Linux uses the dedicated `v0.4.2-arch` release. Download its `PKGBUILD`
into an empty directory and run `makepkg -si`; pacman resolves MPV, FFmpeg and
the required desktop libraries. The installed freedesktop entry appears in the
application menu in KDE, GNOME and XFCE.

## Maintenance

The desktop Settings screen can run local system and playback diagnostics,
create or restore a backup of the library and viewing history, and check GitHub
Releases for updates. Updates download in the background and install only after
the user confirms a restart or exits Pirate Cinema normally.

## Configuration

Copy `.env.example` to `.env.local`. Secrets are intentionally excluded from Git.

- `TORRSERVER_URL`, `TORRSERVER_USERNAME`, `TORRSERVER_PASSWORD` configure TorrServer.
- Единственный источник раздач — встроенный TorrServer с локальной базой `rutor.ls`. Jackett, Bitmagnet, PostgreSQL и Torrents.csv в приложение и установщик не входят.
- `TMDB_READ_TOKEN` (рекомендуется) или `TMDB_API_KEY` включает TMDB-метаданные и постеры. Без ключа работает локальный demo-режим.

## Поиск и добавление

- Поиск использует только встроенный `GET /search` TorrServer с базой Rutor. Внешних резервных источников и отдельного кэша поисковой выдачи нет.
- Кнопка «В медиатеку» отправляет magnet в `POST /torrents` с `action: add`.
- Можно вставить собственную magnet-ссылку через кнопку с магнитом на экране поиска.
- Ответ `/echo` считается успешным по HTTP-ответу и непустой версии, поэтому `MatriX.135` корректно отображается как online.
- Встроенный MPV использует независимый OpenGL-вывод, аппаратное декодирование `d3d11va-copy`, облегчённое масштабирование, VSync и высокий приоритет для более ровного вывода рядом с полноэкранными 3D-приложениями.

## Checks

```powershell
pnpm test
pnpm build
```
