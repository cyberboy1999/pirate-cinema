# Pirate Cinema Desktop

This directory contains the production Rust desktop application. Dioxus Desktop renders the interface; the backend includes the embedded `librqbit` engine, the local FFmpeg/HLS bridge, SQLite history, metadata cache, backup/restore, updater, and tray integration. There is no Electron, Node.js, React, MPV, or TorrServer runtime in this build.

## Development

```sh
cargo test --manifest-path desktop-rust/Cargo.toml --all-targets
cargo clippy --manifest-path desktop-rust/Cargo.toml --all-targets -- -D warnings
cargo run --manifest-path desktop-rust/Cargo.toml --bin pirate-cinema
cargo build --manifest-path desktop-rust/Cargo.toml --release --bin pirate-cinema
```

Windows packaging uses `installer.nsi`. Its stage must contain `pirate-cinema.exe`, `ffmpeg/ffmpeg.exe`, `vc_redist.x64.exe`, `README.md`, `LICENSE`, and `THIRD_PARTY_NOTICES.md`. The release workflow restores the checksum-verified FFmpeg binary and the official Visual C++ Runtime before packaging. Generated packages belong in `local-builds/` and must not be committed.

Linux packaging uses `packaging/build-appimage.sh` with checksum-pinned `linuxdeploy` and `appimagetool`. The AppImage deliberately uses the host distribution's FFmpeg, GTK3, and WebKitGTK 4.1 stack; install the dependencies listed on the repository landing page.

## Data and migration

The production Windows profile is `%LOCALAPPDATA%\Pirate Cinema`. On first launch, 0.6.0 imports data non-destructively from:

- `%APPDATA%\pirate-cinema-desktop` (Electron 0.5.8);
- `%LOCALAPPDATA%\Pirate Cinema Rust` (the retired Rust prototype).

The migration imports per-file history and metadata, preferences, cached posters, and the local search index. Source files are never modified. A marker makes the import idempotent. `PIRATE_CINEMA_DATA_DIR`, `PIRATE_CINEMA_ELECTRON_PROFILE`, and `PIRATE_CINEMA_PROTOTYPE_PROFILE` are test-only path overrides used by isolated installation checks.

The embedded engine keeps torrent metadata under `%LOCALAPPDATA%\Pirate Cinema\rqbit`. Downloaded video pieces are a disposable playback buffer: closing playback stops the stream, FFmpeg, and torrent activity, then removes the disk buffer. A later launch clears leftovers from an abnormal termination. The library and per-file history remain in local SQLite and are never uploaded.
