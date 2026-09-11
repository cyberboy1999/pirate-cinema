# Pirate Cinema — local media library

Локальная медиатека с поиском раздач, постерами и стримингом через TorrServer.

## Run locally

```powershell
pnpm install
pnpm dev:local
```

Open `http://localhost:3000`. The integration API runs on `127.0.0.1:3001` and connects to TorrServer at `http://127.0.0.1:8090` by default.

Metadata is stored in `media.db` under `%APPDATA%\TorrServerDesktop`. When that location is unavailable, development falls back to `local-data/` in the project. Posters are cached beside the database.

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
