# Pirate Cinema Desktop 0.6.6

This directory contains the production Rust desktop application. Dioxus Desktop renders the interface; the backend, TorrServer ownership, MPV IPC, SQLite history, metadata cache, backup/restore, updater, and tray are implemented in Rust. There is no Electron, Node.js, React, or egui runtime in this build.

## Development

```sh
cargo test --manifest-path desktop-rust/Cargo.toml --all-targets
cargo clippy --manifest-path desktop-rust/Cargo.toml --all-targets -- -D warnings
cargo run --manifest-path desktop-rust/Cargo.toml --bin pirate-cinema
cargo build --manifest-path desktop-rust/Cargo.toml --release --bin pirate-cinema
```

Windows packaging uses `installer.nsi`. Its stage must contain `pirate-cinema.exe`, `mpv/`, `torrserver/`, `README.md`, `LICENSE`, and `THIRD_PARTY_NOTICES.md`. Generated packages belong in `local-builds/` and must not be committed.

## Data and migration

The production Windows profile is `%LOCALAPPDATA%\Pirate Cinema`. On first launch, 0.6.0 imports data non-destructively from:

- `%APPDATA%\pirate-cinema-desktop` (Electron 0.5.8);
- `%LOCALAPPDATA%\Pirate Cinema Rust` (the retired Rust prototype).

The migration imports per-file history and metadata, preferences, cached posters, and the local TorrServer allowlist. Source files are never modified. A marker makes the import idempotent. `PIRATE_CINEMA_DATA_DIR`, `PIRATE_CINEMA_ELECTRON_PROFILE`, and `PIRATE_CINEMA_PROTOTYPE_PROFILE` are test-only path overrides used by isolated installation checks.

TorrServer defaults to `http://127.0.0.1:8090`. A responding external server is reused; only a process started and owned by Pirate Cinema is stopped.

See [PARITY.md](PARITY.md) for the Electron 0.5.8 replacement audit.
