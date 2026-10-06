# Pirate Cinema 0.7.2

## Исправления

- Windows-установщик включает официальный Microsoft Visual C++ Redistributable x64 и устанавливает его без диалогов, если он требуется системе.
- Закрытие приложения настраивается: свернуть в трей или завершить работу полностью вместе с дочерними процессами TorrServer и MPV.
- Исправлена сборка Windows NSIS и проверены Windows, Linux и Arch/AppImage пакеты в CI.

## Проверка

- `cargo fmt`, `cargo check`, тесты и `cargo clippy -D warnings` проходят на Windows и Linux;
- AppImage проходит отдельный запуск в Arch Linux.
