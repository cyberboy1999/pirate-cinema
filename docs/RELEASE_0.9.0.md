# Pirate Cinema Desktop 0.9.0

## Русский

- **Проверка раздачи перед просмотром.** В карточке раздачи выберите файл или серию и запустите проверку. Приложение попробует получить и обработать короткий фрагмент, покажет результат и сохранит историю просмотра без изменений. Проверку можно отменить.
- **Блокировка рекламы на RuTube.** В общих настройках можно включить или отключить блокировку и обновить списки uBlock filters — Ads и RU AdList. Приложение поддерживает часть сетевых правил. Реклама, встроенная в само видео, может остаться.
- **Диагностика запуска через VPN.** При тайм-ауте приложение показывает, сколько пиров найдено, сколько подключилось и получены ли данные. Это помогает разобраться со случаями, когда через TUN видны участники раздачи, но загрузка не начинается. Для локального соединения с проигрывателем FFmpeg теперь игнорирует системные переменные HTTP-прокси.
- **Настройки и история.** Настройки записываются атомарно. При миграции истории учитываются последние сохранённые записи SQLite, а при восстановлении резервной копии приложение проверяет формат настроек до замены истории.

**Установка на Windows x64.** Скачайте и запустите файл Pirate-Cinema-Setup-0.9.0-win-x64.exe. FFmpeg входит в комплект. Контрольная сумма установщика приведена в SHA256SUMS.

**Linux x86_64.** Скачайте AppImage, разрешите запуск файла и откройте его.

## English

- **Check a torrent before playback.** Select a file or episode on the torrent details screen and run the check. The app downloads and processes a short sample, then reports the result. You can cancel the check at any time. It does not add a viewing-history entry.
- **RuTube ad blocking.** General settings now let you toggle blocking and update the uBlock filters — Ads and RU AdList lists. The app supports a subset of their network rules. Ads embedded in the video itself may still appear.
- **Troubleshoot playback over a VPN.** Timeout messages show how many peers were discovered, how many connected and whether any data arrived. This helps identify TUN connections where peer discovery works but downloads stall. FFmpeg now ignores HTTP proxy environment variables when connecting to the local player service.
- **Settings and history.** Settings are saved atomically. History migration includes recent committed SQLite entries, and backup restoration validates the settings format before replacing viewing history.

**Install on Windows x64.** Download and run Pirate-Cinema-Setup-0.9.0-win-x64.exe. FFmpeg is included. The installer checksum is in SHA256SUMS.

**Linux x86_64.** Download the AppImage, mark it executable and run it.
