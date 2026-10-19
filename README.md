# Pirate Cinema

**Русский** · [English](#english)

![Главный экран Pirate Cinema](docs/images/pirate-cinema-home.png)

Pirate Cinema — локальное настольное приложение на Rust для поиска, добавления и просмотра раздач. Встроенный librqbit загружает выбранный файл, а ArtPlayer воспроизводит подготовленный FFmpeg HLS-поток внутри приложения. История каждого файла хранится локально в SQLite.

Windows-установщик включает FFmpeg и Visual C++ Runtime. TorrServer, MPV и отдельный GStreamer больше не требуются.

## Возможности

- локальный поиск по индексу RuTor и опциональный Jackett/Prowlarr Torznab;
- добавление `magnet:`-ссылок из приложения и браузера;
- медиатека с постерами, русскими описаниями, фильтрами и сортировкой;
- выбор фильма, сезона, серии и конкретного видеофайла;
- встроенный ArtPlayer: продолжение просмотра, перемотка, аудиодорожка, полноэкранный режим и следующий эпизод;
- отметка «Просмотрено» при первом запуске файла;
- блок «Продолжить просмотр»;
- онлайн-видео и плейлисты: сезоны, серии, качество, автопереход и локальная история;
- сохранённые плейлисты и недавно просмотренные онлайн-видео на главной;
- локальные резервные копии, диагностика, обновления и системный трей;
- русский и английский интерфейс.

Все пользовательские данные остаются на компьютере. Pirate Cinema обращается к Cinemeta, TVmaze и Wikidata только за общедоступными названиями, описаниями и изображениями; история просмотра им не передаётся.

Онлайн-поиск отправляет источнику введённый запрос, а запуск — идентификатор выбранного видео или плейлиста. История остаётся локальной. Для просмотра нужен общедоступный поток; ограничения доступа не обходятся.

## Установка

Готовые пакеты находятся в [последнем релизе](https://github.com/cyberboy1999/pirate-cinema/releases/latest). Все сборки предназначены для 64-битных систем.

### Windows 10/11

Скачайте `Pirate-Cinema-Setup-*-win-x64.exe`. Установщик обновляет существующую установку или заменяет прежнюю Electron-версию, сохраняя пользовательские данные.

### Linux AppImage

AppImage содержит Pirate Cinema, а FFmpeg, GTK и WebKitGTK использует из системы. Перед первым запуском установите зависимости своей системы.

Debian и Ubuntu:

```sh
sudo apt install ffmpeg libgtk-3-0 libwebkit2gtk-4.1-0 libayatana-appindicator3-1 xdotool
```

Fedora, RHEL и совместимые системы:

```sh
sudo dnf install ffmpeg gtk3 webkit2gtk4.1 libayatana-appindicator xdotool
```

Arch Linux и CachyOS:

```sh
sudo pacman -S --needed ffmpeg gtk3 webkit2gtk-4.1 libayatana-appindicator xdotool
```

После установки зависимостей разрешите запуск AppImage:

```sh
chmod +x Pirate-Cinema-*-x86_64.AppImage
```

## Данные и миграция

Основной профиль Rust хранится в `%LOCALAPPDATA%\Pirate Cinema` на Windows и в `$XDG_DATA_HOME/Pirate Cinema` на Linux. При первом запуске выполняется одноразовый импорт из прежней Electron-версии и предыдущего Rust-прототипа. Импорт добавляет только отсутствующие записи и не изменяет исходные базы.

Торрент-движок librqbit работает внутри приложения и открывает только временный read-only HTTP-порт на `127.0.0.1`. При закрытии плеера принадлежащие приложению поток, FFmpeg и загрузка останавливаются.

## Сборка из исходного кода

Установите стабильный Rust toolchain и системные зависимости Dioxus Desktop, затем выполните:

```sh
cargo test --manifest-path desktop-rust/Cargo.toml
cargo clippy --manifest-path desktop-rust/Cargo.toml --all-targets -- -D warnings
cargo build --manifest-path desktop-rust/Cargo.toml --release --bin pirate-cinema
```

Сторонние компоненты и лицензии перечислены в [`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md).

---

## English

Pirate Cinema is a local Rust desktop application for searching, adding, and streaming torrents. Embedded librqbit downloads the selected file and ArtPlayer plays an FFmpeg-generated HLS stream inside the application. Per-file history is stored locally in SQLite.

### Features

- local RuTor index search and optional Jackett/Prowlarr Torznab;
- `magnet:` links from the application or browser;
- a poster library with descriptions, filters, and sorting;
- movie, season, episode, and exact-file selection;
- embedded ArtPlayer resume, seeking, audio-track memory, fullscreen, and next episode;
- launch-based Viewed markers and Continue Watching;
- online videos and playlists with episode selection, quality memory, auto-next, and local history;
- saved playlists and recently watched online videos on Home;
- local backup, diagnostics, updates, and system tray;
- Russian and English interfaces.

Playback history stays on the computer. Cinemeta, TVmaze, and Wikidata receive only public metadata queries, never the local history database.

Online sources receive explicit search queries and selected video/playlist identifiers, not the history database. Playback requires an accessible public stream; access restrictions are not bypassed.

### Installation

Download packages from the [latest release](https://github.com/cyberboy1999/pirate-cinema/releases/latest). Builds target 64-bit systems.

#### Windows 10/11

Run `Pirate-Cinema-Setup-*-win-x64.exe`. It updates an existing installation or replaces the former Electron edition while preserving user data.

#### Linux AppImage

The AppImage contains Pirate Cinema and uses the system FFmpeg, GTK, and WebKitGTK. Install the runtime for your distribution before first launch.

Debian and Ubuntu:

```sh
sudo apt install ffmpeg libgtk-3-0 libwebkit2gtk-4.1-0 libayatana-appindicator3-1 xdotool
```

Fedora, RHEL, and compatible distributions:

```sh
sudo dnf install ffmpeg gtk3 webkit2gtk4.1 libayatana-appindicator xdotool
```

Arch Linux and CachyOS:

```sh
sudo pacman -S --needed ffmpeg gtk3 webkit2gtk-4.1 libayatana-appindicator xdotool
```

Then make the AppImage executable:

```sh
chmod +x Pirate-Cinema-*-x86_64.AppImage
```

### Data migration

The Rust profile lives in `%LOCALAPPDATA%\Pirate Cinema` on Windows and `$XDG_DATA_HOME/Pirate Cinema` on Linux. On first launch it performs a one-time, non-destructive import from the former Electron edition and Rust prototype. Existing source databases are never modified.

The embedded librqbit engine exposes only an ephemeral read-only HTTP port on `127.0.0.1`. Closing playback stops the application-owned stream, FFmpeg process, and torrent activity.

### Development

```sh
cargo test --manifest-path desktop-rust/Cargo.toml
cargo clippy --manifest-path desktop-rust/Cargo.toml --all-targets -- -D warnings
cargo build --manifest-path desktop-rust/Cargo.toml --release --bin pirate-cinema
```

Third-party components and licenses are listed in [`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md).
