# Pirate Cinema — durable project memory

Use this file only after reading `AGENTS.md` and `PROJECT.md`. It records
durable facts that aid a new session; current source and project rules win if
they conflict.

## Product boundaries

- Production desktop application: `desktop-rust/`, written in Rust with Dioxus
  Desktop. Its local data includes SQLite history, settings and poster cache.
- TorrServer is local and configurable, defaulting to `http://127.0.0.1:8090`.
  Pirate Cinema stops only a TorrServer process that it started.
- MPV IPC supplies progress tracking for external playback. The embedded player
  uses local ArtPlayer/hls.js assets and TorrServer GST HLS streams.
- Electron/Node sources are retained for the one-time, non-destructive profile
  migration and as compatibility reference; do not make them the production
  implementation.
- Library data stays local. Public metadata providers may receive only a title
  or public identifier, never the library database, history or credentials.

## Architecture and conventions

- `desktop-rust/src/bin/dioxus.rs` is the Dioxus shell and UI.
- `catalog`, `metadata`, `history`, `settings`, `mpv`, `migration` and
  `torrserver_process` separate public metadata, local persistence, playback,
  migration and owned-process responsibilities.
- Metadata uses no-key Cinemeta, TVmaze and Wikidata. Network failures must not
  block playback; poster-frame fallback remains bounded.
- Preserve per-file SQLite history, MPV IPC behavior, safe migration, and the
  ownership rule for background processes.

## Verified working commands

- Rust format: `cargo fmt --manifest-path desktop-rust/Cargo.toml -- --check`
- Rust checks: `cargo check --manifest-path desktop-rust/Cargo.toml --all-targets --all-features`
- Rust tests: `cargo test --manifest-path desktop-rust/Cargo.toml --all-targets --all-features`
- Rust lint: `cargo clippy --manifest-path desktop-rust/Cargo.toml --all-targets --all-features -- -D warnings`
- Legacy Node checks: `pnpm test` and `pnpm run lint`

## Release and platform facts

- Releases package one Windows NSIS installer and one x86_64 Linux AppImage.
  Do not add retired DEB/RPM/PKGBUILD/web-installer outputs back to releases.
- `release.yml` verifies runtime payload hashes, builds both platforms, and
  runs an Arch smoke test before a tagged GitHub release.
- Windows packaging includes the Visual C++ redistributable. Linux AppImage
  packaging relies on the documented system WebKit/GTK/MPV runtime stack.
- Generated installers, runtime databases, caches, bundled binaries and
  credentials are ignored and must never be committed.

## Recurring pitfalls

- Do not clear local library state after a malformed TorrServer list response.
- Do not stop an external TorrServer that was merely discovered on port 8090.
- Avoid arbitrary metadata waits: public metadata and poster requests are
  bounded, while startup repair runs in the background.
- Treat a runtime, installer or live-provider failure as a debugging task:
  reproduce, inspect boundaries and recent changes, form one hypothesis, then
  add the smallest verified fix.
