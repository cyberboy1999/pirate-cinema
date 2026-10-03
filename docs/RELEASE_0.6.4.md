# Pirate Cinema 0.6.4

Исправлен запуск на Arch Linux и CachyOS: `PKGBUILD` теперь устанавливает обязательный WebKitGTK 4.1. Окно приложения открывается сразу, а комплектный TorrServer запускается и проверяется в фоне, поэтому оболочка рабочего стола больше не должна бесконечно показывать индикатор запуска.

---

Fixed startup on Arch Linux and CachyOS: the PKGBUILD now installs the required WebKitGTK 4.1 runtime. The application window opens immediately while bundled TorrServer starts and is retried in the background, preventing the desktop shell from showing an endless launch spinner.
