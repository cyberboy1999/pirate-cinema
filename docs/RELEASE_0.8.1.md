# Pirate Cinema 0.8.1

## Русский

Pirate Cinema 0.8.1 обновляет настольный интерфейс и работу с обложками, сохраняя полностью локальную архитектуру Rust/Dioxus, встроенный librqbit и ArtPlayer.

### Что изменилось

- Интерфейс переработан в спокойном кинематографичном стиле Apple TV + Plex: крупные постеры, выразительная иерархия, белые основные действия и адаптивные экраны.
- UI разделён на отдельный модуль компонентов и таблицу стилей, чтобы дальнейшие изменения не раздували основной файл приложения.
- Добавлены детерминированные локальные обложки из названия, года, жанров и хеша. Они не требуют сети и не передают медиатеку наружу.
- В настройках появились три режима: «Авто», «Локальные» и «Постеры». Смена режима применяется сразу и не удаляет кэш провайдерских постеров.
- Добавлены локальные списки «Избранное» и «Смотреть позже» с отдельными разделами и быстрыми действиями на карточках.
- Подключены проектные design-skills Impeccable и Frontend Design с лицензиями и зафиксированным происхождением; это инструменты разработки, они не входят в установленное приложение.
- Сохранены изменения 0.8.0: librqbit 9.0.1, исправленный запуск сохранённых раздач, временный буфер воспроизведения и локальный веб-пульт.

### Проверки

- `cargo fmt`, `cargo check --all-targets --all-features` и строгий `clippy -D warnings`.
- 63 детерминированных Rust-теста прошли; 8 публичных сетевых тестов оставлены opt-in.
- Все 6 JavaScript-наборов плеера, смены серий, полноэкранного режима, диагностики, онлайн-плеера и веб-пульта.
- Release-сборка Windows запущена с отдельным профилем: окно `Pirate Cinema 0.8.1` осталось рабочим и визуально проверено.

### Установка

- **Windows 10/11 x64:** `Pirate-Cinema-Setup-0.8.1-win-x64.exe`. FFmpeg и Visual C++ Runtime входят в установщик; нужен Microsoft Edge WebView2 Runtime.
- **Linux x86_64:** `Pirate-Cinema-0.8.1-x86_64.AppImage`. Нужны системные FFmpeg, GTK3, WebKitGTK 4.1, libayatana-appindicator, librsvg и libxdo.
- Проверяйте файлы по `SHA256SUMS`.
- Android-клиент в этот desktop-релиз не входит.
- Windows-установщик не подписан коммерческим сертификатом, поэтому SmartScreen может показать предупреждение.

## English

Pirate Cinema 0.8.1 refreshes the desktop experience and cover handling while retaining the fully local Rust/Dioxus architecture, embedded librqbit, and ArtPlayer.

### Changes

- Redesigned the interface around a calm Apple TV + Plex-inspired cinematic shell with larger artwork, clearer hierarchy, white primary actions, and responsive screens.
- Split the UI into a dedicated component module and stylesheet so future changes do not inflate the application entry point.
- Added deterministic local cover art composed from title, year, genres, and hash. It works without a network request and never uploads library data.
- Added Auto, Local, and Posters cover modes. Switching modes is immediate and never deletes cached provider posters.
- Added local Favorites and Watch Later collections with dedicated views and card actions.
- Added the Impeccable and Frontend Design project skills with their licenses and pinned provenance. They are development tools and are not shipped in the installed application.
- Retained the 0.8.0 engine changes: librqbit 9.0.1, restored-torrent startup fix, disposable playback buffer, and local web remote.

### Verification

- `cargo fmt`, `cargo check --all-targets --all-features`, and strict `clippy -D warnings`.
- 63 deterministic Rust tests passed; 8 public-network tests remain opt-in.
- All 6 JavaScript suites for playback, episode switching, fullscreen, diagnostics, online playback, and the web remote.
- The Windows release build was launched with an isolated profile; the `Pirate Cinema 0.8.1` window remained responsive and passed visual inspection.

### Installation

- **Windows 10/11 x64:** `Pirate-Cinema-Setup-0.8.1-win-x64.exe`. FFmpeg and the Visual C++ Runtime are bundled; Microsoft Edge WebView2 Runtime is required.
- **Linux x86_64:** `Pirate-Cinema-0.8.1-x86_64.AppImage`. System FFmpeg, GTK3, WebKitGTK 4.1, libayatana-appindicator, librsvg, and libxdo are required.
- Verify downloads with `SHA256SUMS`.
- The Android client is not part of this desktop release.
- The Windows installer is not commercially code-signed, so SmartScreen may show a warning.
