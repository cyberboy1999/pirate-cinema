# Electron 0.5.8 → Rust replacement audit

| User flow | Rust desktop |
|---|---|
| TorrServer | Reuses or owns the bundled local process; configurable endpoint; list/add/remove and metadata sync |
| Search | RuTor plus optional Jackett/Prowlarr Torznab, quality filters, validated same-origin torrent downloads, duplicate checks |
| Home | Continue Watching and a two-row Cinemeta catalogue with Russian Wikidata labels and offline cache |
| Library | Equal portrait cards, posters/descriptions, type/status/year/genre filters, sorting, MPV frame fallback |
| Details | Movie/episode grouping, season and specials filters, per-season progress, next unwatched, per-file history, stream copy and diagnostics |
| Playback | Embedded or separate MPV on Windows, separate MPV on Linux, IPC resume, audio/subtitle memory, focus/reuse, multiple windows and EOF-next; optional external player |
| Data | SQLite per-file history, validated backup/restore, one-time non-destructive Electron/prototype migration |
| Settings | RU/EN, player, endpoint, Jackett/Torznab, diagnostics, metadata sync, backup/restore and updater |
| Desktop integration | Native tray hide/open/exit, `magnet:` handler, production shortcuts and application icon |
| Packaging | One production Windows x64 NSIS installer and one Linux x86_64 AppImage |

The production binary is `pirate-cinema`. The retired egui application and separate Prototype installer are removed. Final release acceptance requires a clean Windows installation and an installation over Electron 0.5.8, followed by one real playback/resume check.
