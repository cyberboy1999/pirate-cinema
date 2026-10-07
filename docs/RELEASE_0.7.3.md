# Pirate Cinema 0.7.3

## Исправления

- Windows-установщик теперь включает официальную GST-сборку TorrServer MatriX.144 со встроенным GStreamer. Приложение выбирает её по умолчанию и проверяет `/gst/echo` перед встроенным воспроизведением.
- Главная страница показывает недавно запускавшиеся фильмы и сериалы из локальной истории вместо карусели популярного.
- Убраны лишние кнопки «Продолжить» под постерами медиатеки.
- Обновлены поиск описаний и постеров, обработка названий раздач и фоновое восстановление неполных карточек.
- Сборка релиза запускается одним тегом; обычная отправка ветки больше не дублирует GitHub Actions.

Пользовательские данные, история просмотра и настройки TorrServer при обновлении сохраняются. Linux AppImage по-прежнему использует системные библиотеки, перечисленные в README.

## English

- The Windows installer now includes the official TorrServer MatriX.144 GST binary with its embedded GStreamer runtime. The app selects it by default and verifies `/gst/echo` before embedded playback.
- Home shows recently launched local titles instead of the popular carousel; redundant Continue buttons were removed from library cards.
- Metadata and poster matching, release-title parsing, and background repair of incomplete cards were improved.
- Release Actions now run once per tag rather than also running on branch pushes.

Existing viewing history, settings, and TorrServer data are preserved during updates. Linux AppImage still uses the system libraries listed in README.
