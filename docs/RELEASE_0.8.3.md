# Pirate Cinema 0.8.3 — Linux fix

## Русский

- Linux-updater выбирает AppImage вместо Windows EXE, проверяет ELF и устанавливает право запуска.
- Постеры Cinemeta получили одинаковую рамку 2:3 независимо от исходного изображения.
- Исправлены обрезание названия и кнопок миниплеера; видео сохраняет 16:9.
- Устранены лишние перерисовки при неизменном статусе движка и лишняя генерация обложек при использовании готового постера.
- README дополнен GStreamer-декодерами и командами установки Linux-зависимостей.

## English

- Linux updates select AppImage instead of Windows EXE, validate ELF and set executable permissions.
- Cinemeta posters use a uniform 2:3 frame independent of source dimensions.
- Mini-player titles and controls stay inside the panel; video retains 16:9.
- Avoided unchanged engine-status redraws and unnecessary generated covers when using cached artwork.
- Documented Linux runtime dependencies, including GStreamer codecs.

## Known issue / Известная проблема

Linux online-video playback previously stalled WebKit in WSL. MSE/H.264/AAC
capability checks pass after installing codecs, but complete interactive
playback validation is still required. This build does not claim that the
stall has been fixed.
