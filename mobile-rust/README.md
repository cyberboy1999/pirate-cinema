# Rust Android prototype

Native Rust/Dioxus UI using the existing Rust core, not the legacy Capacitor client.
The separate application ID `com.piratecinema.rust` keeps the existing app and its data intact.

The prototype starts on the online viewing page and uses an in-process torrent engine.
Search, playlists, ArtPlayer controls and online SQLite history reuse the desktop
module. On Android its database stays in app-private storage. No library data is
sent by startup; only an explicit search sends its query to the online provider.

Torrent search now uses an in-process `librqbit` 8.1.1 engine and a local public
RuTor index. The explicit first torrent search downloads the fixed HTTPS index
(about 51 MB); subsequent searches read it locally without transmitting the query
or library. The bundled engine is not a title-search provider itself. Adding a
magnet resolves metadata while paused; choosing Launch explicitly starts the
selected file and opens MPV through the existing per-file history bridge.
Data stays in app-private `rqbit/`; persisted torrents restore paused, and the
read-only streaming endpoint binds only to ephemeral loopback. External MPV can
still cause the OS to background this app; long physical torrent playback on XOS
is not yet verified. librqbit's current stock HTTP handler supports open byte
ranges (`bytes=N-`), not arbitrary closed ranges; no browser/GST parity is claimed.

The dependency is fixed to 8.1.1 with Rust TLS to avoid the additional Android JVM
certificate-verifier integration of the newer branch. Its stock HTTP feature also
requires `tracing-subscriber-utils` and serde `rc`. Windows developer checks can
use `AWS_LC_SYS_PREBUILT_NASM=1` for aws-lc's bundled assembly objects when NASM is
not installed. Use `--locked` to preserve the tested dependency graph.

The Android client uses only the embedded engine. Metadata, posters and per-file
history are preserved; recently
opened torrent cards can be read offline without attempting to launch a stream.

Local diagnostic (explicit isolated profile and public query, no movie download):
`cargo run --no-default-features --features desktop --locked --example rqbit_probe -- target/rqbit-probe "Тачки"`.

For torrent playback, install mpv-android (`is.xyz.mpv`) separately.
The client opens its official release page if it is missing; it never silently installs it.
No external server, fixed port configuration or GST executable is used.

Settings, pending playback identity and per-file SQLite history stay in Android app-private storage.
MPV's returned position is saved when leaving playback. Missing/invalid results never erase history.
Metadata lookup is explicit and discloses the providers receiving the selected title.
Matched descriptions and titles are stored in SQLite, with JPEG posters in app-private
storage. The library uses equal-size 2:3 cards; cached artwork is served locally rather
than downloaded by the WebView on every visit. A failed lookup keeps previous metadata,
and enrichment does not disable playback. The lookup query can be corrected in the card.

## Local build

Validated tooling: Rust Android targets, Dioxus CLI 0.7.10, JDK 21, Android SDK 36,
NDK r27c and Gradle 9.1.0 (the DX-generated wrapper). Set `JAVA_HOME`, `ANDROID_HOME`, `ANDROID_SDK_ROOT`,
`NDK_HOME` and `ANDROID_NDK_HOME` to those installations. On Windows use paths without
spaces (temporary `subst` drive mappings are sufficient).

```powershell
rustup target add x86_64-linux-android aarch64-linux-android
dx build --android --target x86_64-linux-android --no-default-features --features mobile --locked --rustc-args=-Clink-arg=-Wl,-z,max-page-size=16384
```

Repeat with `--target aarch64-linux-android` for ARM64. Both libraries must come from
the same revision before packaging a combined APK. Verify every ELF LOAD alignment
is at least 0x4000 for Android devices with 16 KiB pages.

For a compact local prototype, add `--release` to both DX builds. The Cargo release
profile strips symbols; the native engine is bundled, while MPV is a separate app.
The current DX build still produces `outputs/apk/debug/app-debug.apk`
with a debug signature: optimized Rust code is not production Android signing.

Early local builds temporarily used Gradle 8.14.3 while the generated 9.1.0 wrapper
was downloading. The final combined debug build succeeded with Gradle 9.1.0;
do not rely on patching generated wrapper files during compilation. Initial
tool downloads and debug native packaging can take several minutes.
Never edit generated Kotlin instead of `android/MainActivity.kt`.

```powershell
cargo test --no-default-features --features desktop --locked
cargo clippy --no-default-features --features desktop --locked -- -D warnings
```

The deterministic engine test serves a synthetic four-byte file and checks
repeat launches and open HTTP ranges without downloading public media. Debug APKs are not
production releases: permanent signing, device coverage and desktop feature parity
remain release gates.
