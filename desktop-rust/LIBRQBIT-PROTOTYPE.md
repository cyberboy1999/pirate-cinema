# Local desktop engine prototype

The desktop feature includes the pinned librqbit 9.0.1 engine.
It replaces the external torrent service for search, magnet add, library, video files,
stream URLs and removal. No fixed server port, service binary, restart action or
remote endpoint is used. Existing installed software and old data are not removed.
This change is local-only: published installers/updater payloads are not changed.

Torrent playback stays inside the Dioxus WebView. librqbit exposes only the selected
file on a random loopback port, the bundled FFmpeg creates short on-demand HLS
windows, and the pinned local ArtPlayer/hls.js assets render them. No MPV process,
TorrServer service or fixed media port is used. Per-file progress remains in SQLite.

Selected torrent videos are stored on disk under the profile's rqbit/downloads.
They are not a bounded memory cache. Opening a card can resolve torrent metadata;
adding is paused without payload, and playback selects the chosen file.
Closing ArtPlayer stops its owned FFmpeg child and pauses the last stream lease.
Background artwork lookup
does not resolve torrent files or start video downloads. Session restoration is paused.
Forget preserves downloaded files/history. Old cached SQLite cards are imported once;
their metadata is fetched from peers only when explicitly opening the card.

Search downloads a public index once (about 51 MB), then matches titles locally.
Configured Jackett/Torznab is contacted directly, without the retired service.
Index freshness, torrent-only indexer results without magnet/infohash, arbitrary
closed HTTP ranges and complete public-swarm/device coverage remain limitations.
FFmpeg currently transcodes the selected video to H.264/AAC MPEG-TS in bounded
60-second windows. This favors compatibility over zero-copy playback and can be
CPU-intensive on high-resolution sources.

Backups restore history/settings/posters only. Current engine library/session and
downloaded videos are preserved, not restored from those backups.

Use an isolated profile for checks:

```powershell
$env:PIRATE_CINEMA_DATA_DIR = "ABSOLUTE_PATH_TO_TEST_PROFILE"
$env:AWS_LC_SYS_PREBUILT_NASM = "1"
cargo test --manifest-path desktop-rust/Cargo.toml --lib --bin pirate-cinema --all-features --locked
cargo run --manifest-path desktop-rust/Cargo.toml --example desktop_rqbit_smoke -- OWNED_FIXTURE.torrent OWNED_FIXTURE.mp4
cargo build --manifest-path desktop-rust/Cargo.toml --bin pirate-cinema --locked
```

The smoke test pre-seeds an explicitly supplied owned local fixture and checks the
real chain from librqbit through FFmpeg: manifest/probe, the first MPEG-TS segment,
a segment after a seek, SQLite progress, stream pause and clean loopback shutdown.
It is not evidence of public peer discovery, long-form network playback, every
codec/audio layout or Linux UI behavior.
