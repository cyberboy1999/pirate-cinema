# Pirate Cinema 0.6.2

Исправлен запуск комплектного TorrServer: приложение явно передаёт локальный адрес, порт 8090 и каталог пользовательских данных. При ошибке показывается точная причина, а состояние «TorrServer офлайн» выделено красным.

---

Bundled TorrServer startup now receives the explicit loopback address, port 8090, and user-data directory. Startup failures keep their concrete error visible, and the offline TorrServer state is highlighted in red.
