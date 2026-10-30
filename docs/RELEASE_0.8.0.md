# Pirate Cinema 0.8.0

## Русский

Pirate Cinema 0.8.0 — обновление встроенного torrent-движка и стабильности локального воспроизведения. Приложение остаётся полностью локальным: медиатека, история и настройки не отправляются во внешние сервисы.

### Что изменилось

- Встроенный `librqbit` обновлён до `9.0.1`; loopback HTTP-сервер использует `librqbit-dualstack-sockets 0.7.0`.
- Формат `session.json` и идентификаторы файлов не изменились. Существующие раздачи, история SQLite, постеры, настройки и закладки сохраняются при обновлении.
- Исправлен запуск сохранённой раздачи сразу после открытия приложения: перед изменением выбранного файла приложение ждёт инициализации раздачи до 60 секунд и не переключает другой торрент.
- DHT-персистентность отключена; временный буфер воспроизведения по-прежнему очищается после закрытия плеера и при следующем запуске.
- Сохранены проверенные экран запуска и локальный веб-пульт для ArtPlayer.
- Заголовок окна теперь `Pirate Cinema 0.8.0` — без пометки «локальный прототип».

### Проверки

- 55 детерминированных Rust-тестов по всем целям (40 библиотечных, 11 бинарных и 4 примеров); 8 сетевых тестов доступны отдельно как opt-in.
- `clippy` со строгим `-D warnings` и проверка форматирования.
- Все 6 наборов JavaScript-тестов контроллеров плеера и пульта.
- Сборка Windows NSIS и Linux x86_64 AppImage в GitHub Actions.
- Arch Linux smoke-тест: нативный бинарник и AppImage остаются запущенными не менее 20 секунд.
- Локальный smoke-тест librqbit → FFmpeg/HLS: первый сегмент, перемотка, переключение серии, запись истории SQLite, очистка буфера и чистое завершение.

### Установка

- **Windows 10/11 x64:** запустите `Pirate-Cinema-Setup-0.8.0-win-x64.exe` поверх предыдущей версии. История и настройки сохраняются. Установщик содержит FFmpeg 6.0 и Visual C++ Runtime.
- **Linux x86_64:** используйте `Pirate-Cinema-0.8.0-x86_64.AppImage`. Нужны системные FFmpeg, GTK3, WebKitGTK 4.1, libayatana-appindicator, librsvg и libxdo; точные команды для Debian/Ubuntu, Fedora/RHEL и Arch приведены в README.
- Проверяйте загруженные файлы по `SHA256SUMS`.
- Android-клиент не входит в этот desktop-релиз.
- Установщик Windows не подписан коммерческим сертификатом, поэтому SmartScreen может показать предупреждение.

## English

Pirate Cinema 0.8.0 upgrades the embedded torrent engine and hardens local playback. The application remains fully local: the library, history, and settings are never uploaded to an external service.

### Changes

- Upgraded embedded `librqbit` to `9.0.1`; the loopback HTTP server now uses `librqbit-dualstack-sockets 0.7.0`.
- The `session.json` format and file IDs are unchanged. Existing torrents, SQLite history, posters, settings, and bookmarks survive the upgrade.
- Fixed launching a saved torrent immediately after opening the application: playback now waits up to 60 seconds for the selected torrent to initialize before changing `only_files`, without switching or mutating another torrent.
- DHT persistence is disabled; the temporary playback buffer is still removed after playback and on the next launch.
- The verified startup screen and local ArtPlayer web remote are retained.
- The window title is now `Pirate Cinema 0.8.0`, with no “local prototype” label.

### Verification

- 55 deterministic Rust tests across all targets (40 library, 11 binary and 4 example tests); 8 public-network tests remain available as opt-in checks.
- `clippy` with warnings denied and a formatting check.
- All 6 JavaScript controller suites for the player and web remote.
- Windows NSIS and Linux x86_64 AppImage builds in GitHub Actions.
- Arch Linux smoke test: the native binary and the AppImage both stay alive for at least 20 seconds.
- Local librqbit → FFmpeg/HLS smoke: first segment, seek, episode switch, SQLite history, payload cleanup, and clean shutdown.

### Installation

- **Windows 10/11 x64:** run `Pirate-Cinema-Setup-0.8.0-win-x64.exe` over the previous version. History and settings are preserved. The installer bundles FFmpeg 6.0 and the Visual C++ Runtime.
- **Linux x86_64:** use `Pirate-Cinema-0.8.0-x86_64.AppImage`. System FFmpeg, GTK3, WebKitGTK 4.1, libayatana-appindicator, librsvg, and libxdo are required; exact Debian/Ubuntu, Fedora/RHEL, and Arch commands are in the README.
- Verify downloads with `SHA256SUMS`.
- The Android client is not part of this desktop release.
- The Windows installer is not signed with a commercial certificate, so SmartScreen may show a warning.
