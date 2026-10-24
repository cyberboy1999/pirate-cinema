# Pirate Cinema 0.7.11

## Русский

- Добавлен отдельный локальный веб-пульт для активного ArtPlayer: пауза, перемотка, громкость, следующая серия, полноэкранный режим и выбор озвучки.
- QR-код и адрес пульта доступны в настройках приложения и через пункт «Веб-пульт» в настройках ArtPlayer.
- Пульт выключен по умолчанию, защищён случайным локальным токеном и не даёт доступа к медиатеке, поиску или magnet-ссылкам.
- Остановка пульта теперь немедленно разрывает зависшее локальное подключение; файл токена в Linux доступен только владельцу.
- Добавлен короткий фирменный экран запуска.
- Улучшено восстановление недоступных постеров по уже проверенному IMDb ID без поиска по произвольному названию.
- README дополнен русской и английской инструкцией по запуску веб-пульта.

## English

- Added a separate local Web remote for the active ArtPlayer session: pause, seek, volume, next episode, fullscreen, and audio-track selection.
- The QR code and remote URL are available in application Settings and through the Web remote entry in ArtPlayer settings.
- The remote is disabled by default, protected by a random local token, and cannot access the library, search, or magnet links.
- Stopping the remote now immediately closes a stalled local connection; its Linux token file is owner-only.
- Added a short branded startup screen.
- Improved unavailable-poster recovery through an already verified IMDb ID without arbitrary title search.
- Added Russian and English Web remote setup instructions to README.
