# Состав Pirate Cinema Desktop 0.8.1

## Архитектура

```text
Dioxus Desktop UI
  -> Rust core
  -> embedded librqbit
  -> loopback HTTP Range stream
  -> local FFmpeg/HLS bridge
  -> ArtPlayer + hls.js
  -> SQLite history and metadata
```

## Каталоги

- `desktop-rust/src/bin/dioxus.rs` — UI, навигация и связь с WebView/ArtPlayer.
- `desktop-rust/src/desktop_engine.rs` — встроенный `librqbit`, торрент-сессии и локальный поток.
- `desktop-rust/src/web_stream.rs` — FFmpeg, HLS, выбор качества/аудио/субтитров.
- `desktop-rust/src/history.rs` — SQLite-медиатека и история по каждому файлу.
- `desktop-rust/src/catalog.rs`, `desktop-rust/src/metadata/` — постеры, описания и confidence-проверки.
- `desktop-rust/src/remote_control.rs` — опциональный LAN-пульт с QR-кодом и токеном.
- `desktop-rust/src/rutube_prototype.rs` — локальная история и онлайн-воспроизведение.
- `desktop-rust/assets/` — закреплённые ArtPlayer, hls.js, JS-контроллеры и иконки.
- `desktop-rust/tests/` — JavaScript-тесты состояния плеера и пульта.
- `desktop-rust/examples/` — диагностические Rust-примеры.
- `desktop-rust/vendor/` — полный набор Cargo crate sources из `Cargo.lock`.
- `runtime/windows/` — FFmpeg и VC++ Redistributable для Windows-пакета.
- `tools/nasm-3.02/` — переносимый NASM для сборки `aws-lc-sys`/`librqbit` на Windows.
- `tools/nsis-3.13/` — переносимый NSIS для сборки Windows-инсталлятора.
- `artifacts/` — проверенные установщики Windows и Linux AppImage.
- `build/icon.ico`, `public/favicon.png` — ресурсы установщика и локального веб-пульта.
- `docs/RELEASE_0.8.0.md` — описание опубликованного релиза.
- `.agents/skills/impeccable/` — проектный design-skill Impeccable 4.8.0 (Apache-2.0).
- `.agents/skills/frontend-design/` — проектный skill frontend-design (Apache-2.0).
- `.opencode/commands/impeccable.md` — команда `/impeccable` для OpenCode.
- `.impeccable/config.json` — проектная конфигурация Impeccable с отключённой телеметрией.

## Design skills

В проекте подключены два instruction-only skill из `.agents/skills/`; их
находят OpenCode и Codex:

- `impeccable` — дизайн-направление, критика, аудит, полировка, типографика,
  движение, доступность и адаптивность. Вызов: `/impeccable` в OpenCode или
  инструмент `skill` с именем `impeccable`.
- `frontend-design` — выразительный production-grade frontend/UI и визуальная
  проверка в браузере. Вызов: инструмент `skill` с именем `frontend-design`.

Исходники, commit-хэши и лицензии записаны в `UPSTREAM.md` и `NOTICE.md`
внутри каталогов skill. Движок Impeccable не хранится в репозитории: при
первом запуске `scripts/impeccable.cmd` (Windows) или `scripts/impeccable`
(POSIX) скачивает и проверяет бинарник в `~/.impeccable/bin`. Каталоги
`scripts/bin/` и runtime-состояние `.impeccable` исключены из Git.

## Встроенные зависимости

- Rust crate sources, зафиксированные `Cargo.lock`;
- `librqbit 9.0.1`;
- Dioxus Desktop 0.7;
- ArtPlayer 5.4.0 и hls.js 1.7.3;
- SQLite через bundled `rusqlite`;
- FFmpeg для Windows runtime;
- VC++ Redistributable для установки Windows.

## Системные зависимости

Их нельзя корректно переносить простым копированием папки:

- Windows: Rust stable MSVC, Visual Studio C++ Build Tools и WebView2 Runtime; NASM и NSIS уже включены;
- Linux: Rust stable, NASM, FFmpeg, GTK3, WebKitGTK 4.1, libayatana-appindicator, librsvg, libxdo, OpenSSL development files;
- Linux-системные библиотеки устанавливаются пакетным менеджером и не переносятся из Windows-папки.

## Что намеренно исключено

- Android, Electron, Node/React server и старые UI-прототипы;
- TorrServer и MPV;
- `target/`, `node_modules/` и прочие пересобираемые каталоги;
- пользовательские SQLite-базы, история, настройки, торрент-данные и постерный кэш;
- ключи, токены, `.env` и signing secrets;
- smoke-test профили и временные HLS-сегменты;
- бинарники движка Impeccable (`scripts/bin/`), hooks, provider-деревья и runtime-состояние `.impeccable`.
