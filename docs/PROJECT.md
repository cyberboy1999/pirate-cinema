# Pirate Cinema — project context

2026-10-20 payload-cache lifecycle: librqbit still uses its stable filesystem
storage so the OS can cache hot pieces in RAM, but downloaded video data is now
treated as disposable playback buffer. The last stream lease pauses the torrent
and deletes only its `rqbit/downloads/<hash>` payload; persisted torrent metadata
stays available for replay, including imported/private `.torrent` files. Startup
also clears `rqbit/downloads`, covering crashes and forced termination without
using librqbit's experimental unbounded in-memory example storage.

2026-10-20 desktop online-page redesign: the optional online page now follows the
local media-library hierarchy with one compact search header, a horizontal local
Continue Watching rail, poster-shaped saved playlists and a clean result grid.
Playlist removal and history maintenance remain available without occupying the
primary browsing surface. Search, playlist loading, SQLite history, ArtPlayer,
quality selection, auto-next and fullscreen behavior are unchanged.

2026-10-20 desktop legacy cleanup: the production desktop settings model no longer
contains a TorrServer endpoint, MPV selection or an embedded-player compatibility
flag. Saving settings removes those retired JSON keys while preserving language,
close behavior and direct Jackett/Torznab credentials. The unused TorrServer process
manager and MPV IPC modules were deleted; the active desktop path is the embedded
librqbit engine, local index/direct Torznab search and ArtPlayer HLS playback. Legacy
profile migration still recognizes old data names so upgrades remain non-destructive.

2026-10-18 desktop ArtPlayer/librqbit local prototype: the desktop prototype no
longer invokes MPV or TorrServer. The shared in-process librqbit engine exposes the
selected file on an ephemeral read-only loopback endpoint; bundled FFmpeg produces
bounded on-demand HLS windows, and the existing pinned ArtPlayer/hls.js UI keeps
episode selection, audio selection, progress, auto-next and fullscreen inside the
application. Closing playback kills the owned encoder and releases the final swarm
lease. The isolated owned-fixture smoke passed manifest/probe, first and post-seek
MPEG-TS segments, SQLite history and clean shutdown. Published packages and updater
payloads are unchanged; this remains a local prototype.

2026-10-19 desktop lifecycle hardening: the owned librqbit loopback listener is
verified ready before FFmpeg receives its URL. Playback preparation is generation-
fenced so Close/shutdown cancels an in-flight probe, stale manifest requests cannot
retire a newer episode, and failed history persistence releases the pending stream.
Episode replacement keeps the current ArtPlayer alive until the new manifest is
accepted. Close removes owned encoders, torrent leases and HLS directories; one-time
startup cleanup removes cache left by an abnormal prior exit. The owned-fixture smoke
also races an in-progress segment request against Close and verifies the loopback
server and swarm stop. This is still local prototype work, not a published release.

## Rust Android migration — publication gate (2026-10-17)

2026-10-18 Android removal follow-up (supersedes the historical opt-in notes below): removed external TorrServe support entirely from the Rust Android client, including settings, package queries, discovery, health checks and all action fallbacks. Android torrents now use only embedded librqbit with separate MPV. Shared desktop-only server, migration, updater and backup entry points are excluded from Android compilation; Windows/Linux behavior is unchanged. Old profile settings are ignored, not deleted; SQLite history, posters and the search index are preserved. No separately installed application was removed.

Verified Settings and repeat Cars/Тачки search on the physical Infinix after installation: 60 results, first Cars (2006), 4.37 GB, 32 indexed seeds. No torrent payload was added or played in this check. Mobile deterministic tests: 12 passed, two ignored; shared core: 57 passed, six ignored. Mobile preview clippy with warnings denied and ArtPlayer JS tests passed. Both ARM64/x86_64 production builds passed. APK signature and ZIP alignment passed; every native LOAD segment is aligned to 16384 bytes. Neither native library contains retired server names/package strings, and the manifest queries only MPV. Android-only test cfg boundaries were also corrected; cross-target test execution is not claimed.

Current local APK: `local-builds/Pirate-Cinema-Rust-Android-Prototype.apk`, 43,979,212 bytes, SHA-256 `214c271075e219cc91d47cc51fb65a66075f60615afc5a1f36612f7d616548c5`. Installed over the existing phone prototype without clearing its data. Still debug-signed and local-only; no publication or signing-key action.

2026-10-18 librqbit local prototype: added pinned in-process librqbit 8.1.1 (Rust TLS), sharing the existing external MPV/SQLite bridge. Embedded mode is used when external TorrServe is disabled. Explicit torrent Search downloads a fixed HTTPS public RuTor index once (observed 50,949,782 bytes), then performs word-based matching locally with streaming deflate/JSON parsing, bounded decompressed input and at most 121 retained candidates before trimming to 60. No search query/library/history is uploaded by this index path. The separate engine uses app-private rqbit/ data and a read-only ephemeral 127.0.0.1 HTTP endpoint. Add resolves metadata paused with no selected payload files; Play validates the real file, maps shared one-based IDs to zero-based engine IDs, and selects just that file. Persisted sessions are rewritten atomically to paused before engine restoration; repeat playback does not restart an already-live handle. The owned HTTP task and session cancellation token are cancelled on Engine drop.

Verified actual Cars/Тачки search both through an isolated Windows diagnostic and the physical Infinix Android UI with external TorrServe disabled: 60 results, first Cars (2006) BDRip 1080p, 4.37 GB, 32 seeds according to the downloaded index (not a live swarm measurement). No Cars torrent was added or played. All 11 deterministic mobile tests passed (two public online tests remain ignored in that run), clippy with warnings denied passed. A synthetic four-byte torrent regression verifies repeated launch, original file ID mapping and HTTP 206 for open Range bytes=1- without public media. The stock librqbit8 HTTP handler ignores closed ranges; arbitrary closed-range/browser compatibility is not claimed. Independent review's restart/autodownload findings were fixed and re-reviewed with no remaining Important finding for the local search scope. Physical MPV torrent playback/background survival on XOS remains unverified; Android online playback evidence above is separate. No release/push/signing-key action occurred.

Final local librqbit APK: 44,177,998 bytes, SHA-256 `3a81ea8b397750a79516e99ba2e55d3c72901ca3b78b8cee41740705aef1b717`, replacing `local-builds/Pirate-Cinema-Rust-Android-Prototype.apk`. Both ARM64 and x86_64 were built from the same source; signature, ZIP alignment and all ELF LOAD segment alignments (at least 0x4000) passed. Installed over the Infinix prototype without deleting history, posters or the search index. Still debug-signed and local-only, not a production Android release.

2026-10-18 follow-up: user deferred Android TorrServe integration. Mobile startup now defaults to the existing online ArtPlayer/search/playlist/history component, sharing its source rather than adding a second player. Android online history uses the app-private profile; desktop history location is unchanged. TorrServe startup/search/add actions are behind a persisted explicit opt-in, default off even for existing profiles; stored endpoints and history are not removed. Offline recent torrent cards are deduplicated from SQLite and retain cached descriptions/posters. Mobile state/cache/online tests passed (8), both explicitly run public playlist/fragment tests passed, shared-core tests passed (57, six ignored), mobile desktop-preview clippy and ArtPlayer JS checks passed. Independent read-only review found no Critical/Important issue in this delta, while explicitly leaving device verification to live checks.

Physical Infinix check of the online delta: the supplied public playlist returned 11 episodes, selecting the first opened the dedicated ArtPlayer viewing page, and a manual Play displayed changing video frames. Back to Search saved a recently watched card; force-stop/relaunch retained it, and reopening restored exactly 00:43 / 22:01. Fullscreen entered a portrait letterboxed player; forced landscape, audible output, quality/episode switching, EOF and network-loss behavior are not claimed as live-verified. Final mobile CSS keeps all four navigation tabs visible and stacks the narrow-screen heading; final installed APK opened successfully and the offline library still showed the original Big Buck Bunny cached poster without a server. Both ARM64 and x86_64 were rebuilt from the same final source, signature verified, with all four ELF LOAD segments per library aligned at least 0x4000. Local APK is 38,911,864 bytes, SHA-256 `d1f007bef10e660d97d3b045777d95a75a0e9443826aee7715d4ca5358ede39a`, at `local-builds/Pirate-Cinema-Rust-Android-Prototype.apk`. It remains debug-signed, not a production/universal 32-bit Android release; no GitHub publish or signing-key transfer occurred.

2026-10-18 approved PC/USB physical-device test passed sampled playback: started the installed Windows GST binary (`C:\Program Files\Pirate Cinema\torrserver\TorrServer-gst-windows-amd64.exe`, MatriX.144) on loopback 8091 with a separate ignored `mobile-rust/target/usb-server-profile`, not the user's PC library. An explicit ADB reverse mapped the phone's 8091 to the PC's 8091; the Android settings UI temporarily saved that endpoint and reported Connected/MatriX.144. Using the physical Infinix UI, added the public Big Buck Bunny magnet, refreshed the library, opened original video file ID 2, explicitly fetched its JPEG poster and Russian Wikipedia description, and launched MPV successfully. After more than a minute of observed changing video frames, returning from MPV saved 01:27. Force-stopping/relaunching Rust retained the poster, description and 01:27 position; a repeat launch continued playback and returning saved 02:13. No full-film, audible-output, audio-track, series/EOF or network-outage coverage is claimed. This validates the ARM64 client against a PC server, not standalone Android TorrServe on this XOS firmware. No new APK/source feature/release/signing action was needed.

Cleanup completed after the phone reconnected: restored `http://127.0.0.1:8090` through the prototype settings and verified `files/settings.json` through ADB. `adb reverse --list` is empty, so no test tunnel remains. The test server PID 14440 no longer exists and PC port 8091 has no listener; no unrelated process was stopped. Local poster/history and the separate PC test profile were preserved. Successful PC/USB playback does not resolve the separate XOS background-freezing blocker for standalone phone TorrServe.

2026-10-18 explicitly approved temporary global Hiber-policy test: captured `is_enabled_hiber=true`, issued only `-policy -switch 1 false`, foregrounded TorrServe once and returned to Rust. Although the command printed `PolicyId:1 PolicyType:POLICY_HIBER State:false`, after twelve seconds the actual engine still reported `is_enabled_hiber=true`, the TorrServe cgroup was `frozen 1`, and independent curl timed out. Immediately issued `-policy -switch 1 true` and verified `is_enabled_hiber=true` again. Thus printed policy-switch acknowledgement is not proof of engine disable. Static inspection confirms direct `-hiber disable` and `enable_hiber` commands are gated by the same production-build restriction as the per-UID whitelist. Do not claim this workaround succeeded. The entire USF package (which also handles thermal/privacy/other policies), system security and root/debug settings were not disabled or changed. A PC/USB-backed server would be a separate, explicitly disclosed test arrangement, not standalone Android verification.

2026-10-18 USF research (read-only): device build is X6853-16.3.0.145SP06(RU005PF001AZ), `com.transsion.usf` 25.11.24.001 / service 35.1.0.103. The service exposes read-only help through `dumpsys activity service com.transsion.usf/.UsfMainService -h` and `-help -hiber`; the native `dumpsys hiber --help` returns FAILED_TRANSACTION. Static inspection of the local system APK (not user data) found `add_runtime_white_app <uid> <boolean>` and `set_app_mode`, but both are guarded by PermissionUtils.isDebuggable: root-support or a `userdebug` build is required. This phone reports build type `user`, root support unset. Read-only queries returned `is_runtime_white_app: uid=10283, result=false` and `get_app_mode: uid=10283, mode=-1`. No whitelist writes, debug/root flags, global disable commands, Binder transactions or config replacement were attempted. Phone Master/App launch remains an unverified UI route for a supported OEM-specific exception; its presence does not prove that it will alter USF. Do not suggest the guarded shell commands as a verified production workaround.

2026-10-18 physical-device restart check: the user fully restarted TorrServe after setting battery exemption and locking both apps in Recents. Its new process (14066) still changed from `frozen 0` with a working `/echo` to `frozen 1` and a four-second HTTP timeout after returning to Rust. System log identified the actor explicitly: `Usf_Hiber/stateManager: freeze uid: 10283 ru.yourok.torrserve`, followed by `Usf_Hiber/hiber: idleUids` about five seconds after the activity was paused. This establishes vendor USF hibernation as the observed suspension source; ordinary battery exemption and Recents locking did not prevent it on this device. No USF service, global freezer or security policy was disabled. Physical playback remains blocked, not verified.

2026-10-18 physical-device retry after the user's background-permission change: one `/echo` probe initially passed with Rust foreground, but another explicit Add attempt failed at the server preflight, and independent USB-forwarded curl also timed out. Read-only inspection found a concrete OS-level suspension: `/sys/fs/cgroup/apps/uid_10283/pid_12423/cgroup.events` reported `frozen 1` while Android Activity Manager reported `isFrozen=false`, `cached=false` and an active foreground service. Bringing the existing TorrServe activity to the front changed the same cgroup to `frozen 0` and `/echo` immediately returned MatriX.147. Returning to Rust and waiting six seconds reproduced `frozen 1` plus a bounded four-second curl timeout. Thus OEM/cgroup suspension, not a dead server or just a short Rust timeout, blocks this physical-device test. No freezer flag, battery policy or security setting was written by automation. The direct-phone netcat diagnostic did not terminate as expected and was interrupted; it supplies no HTTP success evidence. Playback/history on this phone still cannot be claimed as verified.

2026-10-18 size/physical-device follow-up: a Cargo release profile strips native symbols. DX `build --release` for ARM64 and x86_64 produced a combined local APK of 27,135,671 bytes (25.88 MiB), down from 247,690,196 bytes (236.22 MiB). Both ELF libraries retain 0x4000 LOAD alignment and the APK signature verifies. DX still packages this local build as a debug-signed Android APK, not a permanently signed production release. The same root prototype path now contains SHA-256 `9a760e4400ef5bfb66cb0b73d142034ad34713c42f93c5f81baf339c812a8f27`. The emulator upgraded without losing its cached poster. On the user-connected Infinix X6853 (Android 16, ARM64), the optimized APK installed and opened without an observed crash; settings navigation, scrolling, dependency detection and the Open TorrServe intent worked. Separately installed official TorrServe client 143 (verified against the upstream SHA-256) and ARM64 MPV were absent initially. The user handled the OS permission prompt and brought up local server MatriX.147. Rust startup discovered port 8090 automatically. Physical playback/history checks remain incomplete: `/echo` responds immediately with TorrServe foreground, but repeatedly times out shortly after returning to Rust, even via an independent USB-forwarded HTTP request. The TorrServe process and NotificationTS foreground service remain present (`isForeground=true`); background suspension is suspected, not conclusively proven. The test magnet Add failed at the preflight read, and the foreground server still reported an empty library: do not claim playback or addition passed. User asked to allow background operation manually before repeating. No permission/security/battery setting was changed by automation, and no release/key/Secrets action was performed.

2026-10-18 local follow-up: Android enrichment now reuses the shared SQLite metadata table and stores hash-named JPEGs in app-private `posters/`. A Dioxus asset handler exposes only canonical hash JPEG routes, with a five-MiB bound and no arbitrary file access; cards use a uniform 2:3 poster grid and show cached descriptions on reopening without contacting providers. Explicit lookup discloses Cinemeta, TVmaze, Wikidata and Wikipedia, has an editable title query, and uses a separate loading flag so playback remains available. Missing/error lookups retain prior data; poster download failure can still save descriptions. Provider errors, including partial outages with no match elsewhere, are no longer silently converted to no-match. A diagnostic example accepts only an explicitly supplied public title and does not read library data. Wikipedia artwork fallback requires the article's Wikidata P345 to match the selected IMDb ID before accepting its image or Russian summary. A failed Cinemeta image download for Big Buck Bunny was recovered through this verified fallback (303450-byte JPEG); Rick and Morty returned a 36831-byte JPEG. Sintel still had no poster in the sampled lookup: this is not a claim of complete provider coverage.

Final live Android verification: explicitly looked up Big Buck Bunny, displayed its poster and Russian description, force-stopped/relaunched the Rust app, and confirmed the local poster in the library and description in the reopened card without another lookup. Playback resumed from 06:31 through the separate MPV app; leaving it saved 07:14. Shared-core tests passed (57, six live/runtime tests ignored), three mobile state/cache tests passed, and mobile preview clippy with warnings denied, formatting and diff checks passed. ARM64 and x86_64 were rebuilt from the same source; the combined debug APK's signature and every ELF LOAD alignment (0x4000) were verified. The same local artifact was replaced at `local-builds/Pirate-Cinema-Rust-Android-Prototype.apk`, SHA-256 `60b9151092e57931f80c8ee6cf0487e9a5c52252aa9cec368ccc8ab3d372cd0d`. Only x86_64 was exercised on an emulator; physical ARM64 verification, release signing and full feature parity remain unfinished. No release, permanent signing-key creation or Secrets transfer was performed.

The user explicitly superseded publishing the Capacitor Android client: migrate Android to the Rust version first, then publish a joint Windows/Linux/Android release. Android uses separately installed regular TorrServe and mpv-android; GST remains Windows-only packaging policy, not an Android requirement. Existing Capacitor sources and user changes stay intact as the reference implementation. No new version tag, signing key or GitHub Secrets transfer has been performed for this migration; explicit permission for transferring the future Android key/password to cyberboy1999/pirate-cinema Secrets was received.

`mobile-rust/` is now a real Rust/Dioxus Android client sharing `desktop-rust` without desktop-only dependencies. The native JNI bridge uses app-private settings/SQLite and explicit external TorrServe/MPV intents. Pending playback identity survives process recreation. Metadata lookup requires an explicit disclosed action; library data is not automatically sent to providers. Startup refreshes the saved local endpoint once. The separate prototype ID `com.piratecinema.rust` preserves the existing Capacitor installation and its data.

A debug APK compiled with Dioxus CLI 0.7.10, Java 21 and NDK r27c was installed and launched on the Android emulator with 16 KiB pages. All native ELF LOAD segments use 0x4000 alignment (DX needs the explicit `--rustc-args=-Clink-arg=-Wl,-z,max-page-size=16384` flag). Early local builds temporarily used Gradle 8.14.3 while the generated 9.1.0 wrapper downloaded; the final combined debug build succeeded with Gradle 9.1.0. Do not keep racing edits to generated wrapper files as a build procedure.

Live Android checks confirmed native HTTP library/file loading, installed-app detection, MPV playback of a synthetic HTTP range-served clip, a returned 00:36 position saved in SQLite, survival across force-stop/relaunch, and playback resuming near that position. EOF without position preserves history rather than inventing progress. Two mobile state tests, desktop-preview clippy with warnings denied and the shared core's 54 tests passed (six live/runtime tests ignored). Local HTTP fixtures require execution outside the network-restricted sandbox. Physical ARM64 device verification, Android UX parity, permanent signing, production packaging and the joint release remain unfinished. Do not equate the working debug prototype with a universal production release.

Follow-up live verification: the Rust app automatically loaded the real Android TorrServe MatriX.147 library on startup, opened original video file ID 2 from the public Big Buck Bunny torrent, played it for over six minutes, and saved the returned 06:14 position. ARM64 Rust/Kotlin/APK compilation also succeeded; this is cross-compilation, not a physical ARM64 device test. Playback adds a per-launch URL session marker echoed by MPV, rejects late callbacks after abandoning a launch, blocks overlapping launches, persists pending identity, and retries SQLite failures without discarding the returned result. An explicit Cancel Waiting action preserves history and never kills the external player. Repeat native verification with the new URI/JNI bridge resumed the film and saved 06:31 successfully. Review found no new Critical/Important blocker for the local prototype, but noted corrupted-marker recovery and the single callback slot as remaining minor resilience limitations.

The previous combined ARM64+x86_64 debug APK was signature-verified, installed over the previous prototype without deleting history, and launched successfully; it has now been replaced by the verified 2026-10-18 artifact above. APKs and build targets are ignored, never committed. The synthetic fixture server was stopped after testing. The separately installed TorrServe/MPV and the existing Capacitor app/profile were not removed.

Stock Android TorrServe disables `EnableRutorSearch`, causing `/search` to return HTTP 400 with `[]`. The explicit Search action now prepares only that flag through a read/merge/write settings request, preserving cache and Torznab configuration; already-enabled servers are not rewritten. Initial indexing downloads the official public index (about 51 MB in this test), so results may not be immediate. Actual Android API verification reached HTTP 200 and nonempty Sintel results after indexing. The app UI also successfully added the official Sintel magnet and opened its original video file. A settings-preservation regression brings shared-core tests to 55 passed, six ignored. Metadata lookup for the short film returned no match; poster/description enrichment is not yet verified as working on Android, and no parity/release-ready claim is made.

Final installed APK UI check: started with `EnableRutorSearch=false`, tapped Find, received the initial-index message instead of HTTP 400, then repeated Find and saw seven Sintel results with Add actions. The app process remained running. No GitHub push, tag, release, permanent key generation or Secrets transfer was performed during this prototype step.

Windows packaging rule: include only `TorrServer-gst-windows-amd64.exe`; never include the regular/non-GST TorrServer, including as a fallback. Applies to future Windows release builds, not Linux packages.

The Rust bundled server now uses loopback port 8091. Startup migrates the former exact default URL 127.0.0.1:8090 to 8091; custom endpoints and TORRSERVER_URL overrides remain unchanged. A compact Restart action below the status indicator retries startup or restarts only our owned child, refuses external servers, and serializes against initial startup. Windows executable resolution and release staging are GST-only. Restart interrupts active streams but preserves the profile/database.

Local prototype diagnostics capture bounded stdout/stderr (32 chunks of 2 KiB each per stream combined) in memory for the last owned launch, with EXE, working directory, fixed arguments, UTC start timestamp, PID and exit status. Settings probes /echo and /gst/echo independently without configuring GST. Windows port 8091 inspection reports listener PID and owned/external classification using a bounded, hidden read-only PowerShell query. Copy Diagnostics exports a separate allowlisted report with app/OS/server/WebView user-agent versions and technical statuses; it excludes raw logs, profile paths, custom endpoint hostnames, keys and library/history data. No report is sent automatically.

GST diagnostics also validate gstreamer.works and gst_discoverer.works, so HTTP 200 alone is not success. `node desktop-rust/tests/diagnostics-copy.cjs` exercises the actual embedded clipboard handler's native, fallback and permission-denied paths. Rust tests cover the bounded log/private-export boundary and broken GST replies; the ignored Windows port inspection and real bundled GST start/restart checks passed locally.

Windows GST startup retries only the specific random peer-port bind denial caused by Windows excluded UDP ranges. The retry selects a UDP/TCP-bindable port through the OS and supplies `--torrentaddr` without changing saved TorrServer settings or explicit fixed peer ports; three attempts share the existing 60-second deadline. The ignored `recovers_from_windows_excluded_peer_port` test accepts a reserved port through `PIRATE_TEST_BLOCKED_PEER_PORT` and exercises the real GST binary. Both restart buttons immediately invalidate the old online indicator. A bounded `/echo` health check runs every five seconds while idle, preserves library data and unrelated errors, and rejects replies from before a restart using an operation generation.

## Development discipline

Codex task lifecycle is: understand the request and project rules; recall
`PROJECT_MEMORY.md`; inspect current code; apply the relevant engineering
workflow; implement; test and verify; use Ponytail to remove unjustified
complexity; then record only durable, confirmed new knowledge. Current source
and project rules override memory. The repository keeps shared project memory;
Codex session context remains local and is not committed.

Latest published release before this work: 0.7.7. Version 0.7.8 keeps the single Windows NSIS installer and Linux x86_64 AppImage model. It refreshes the online-viewing screen and removes the retired TorrServer/MPV desktop modules and settings. Windows bundles the checksum-verified FFmpeg executable; Linux uses system FFmpeg. Fresh installs package no runtime database, cache or user history.

Release 0.7.6 preparation enables the online module by default (the existing feature name is retained for compatibility), adds its JS regressions to both CI platforms, and bumps Cargo/lock/NSIS versions consistently. Public release notes/README describe online playback without source-specific branding. Live Windows checks confirmed saved playlists, compact episode selection, a manual next-episode click, resolution retained at 360p on the new episode, disabled auto-next stopping at the end, Viewed and From Start actions, and successful opening from Home after restarting the process. A disappearance observed during earlier user-input interference could not be reproduced; the repeat launch captured no stderr error. Deliberate OS/network settings changes were not used to induce an outage; real-network outage recovery remains covered by automated fault tests rather than a live disconnection claim.

Final Windows 0.7.6 restart check restored episode three at exactly 07:10 / 21:55 from Home, with 360p and auto-next disabled still selected. The saved playlist and three history records remained available, and the playlist reloaded all 11 entries. All-target/all-feature Rust checks passed (68 tests, 12 ignored live/runtime checks); clippy with warnings denied, formatting, five Node regression scripts and the default release build passed. The earlier local-only/opt-in notes below describe prototype development, not the default configuration of 0.7.6.

Online prototype follow-up (local-only): the user confirmed history survives a full restart. Episode settings now group explicitly recognized Russian `N сезон M серия` / `SxxEyy` titles by season with short series labels; unrecognized entries remain numbered in provider order under No Season. Quality is persisted as resolution, remapped for each manifest, with Auto fallback if unavailable. A next-episode control appears in the final 120 seconds and a persisted auto-next switch gates end-of-video transitions. Fatal HLS failures expose Retry and allow at most two delayed automatic attempts while visible; retry resolves a fresh public HLS URL and preserves pending resume position, pause intent and the existing player/fullscreen. Retry timers are cancelled on switching/destroy. History controls in the online page offer Continue, From Start, Viewed and per-entry removal; saved playlist links can be reopened/removed without resubmitting a URL. All state remains in separate rutube-history.sqlite tables; no new dependencies or release changes. SQLite tests cover preference/history/bookmark editing and reopen; JS tests cover season selection, resolution remapping/fallback, next countdown/toggle, retry bounds, pending-resume recovery and timer cleanup. Build/clippy and existing player regression checks passed. These latest controls/recovery scenarios still require live WebView/network-loss UI verification; mocked checks are not a substitute.

The opt-in online prototype accepts public `https://rutube.ru/plst/<numeric-id>/` links in its search field. It loads up to 25 API pages, deduplicates video IDs while preserving provider order, and exposes escaped episode labels in the existing ArtPlayer settings. Selection and end-of-video advance resolve only members of the loaded queue, reuse the player/HLS instance and retain fullscreen; hidden/native-minimized windows do not autoplay. No season numbers are inferred from arbitrary video titles. Oversized playlists fail explicitly rather than silently omitting later pages. This is local-only, not part of the published release.

2026-10-17 Windows UI check of user-provided playlist 359607: loaded all 11 ordered season-one entries; first episode played with advancing timer; manual selection reached episode five while retaining native fullscreen; manual quality selection (480p) retained playback position; seeking near the end triggered episode six at 00:00 without exiting fullscreen. Minimizing near episode-six completion prevented episode seven from starting, confirmed after restoring the window at 21:07/21:07. Back to Search retained the playlist URL/results. Existing player-state, switch, controller and Rutube Node tests passed. This was sampled playback, not full-length viewing of all 11 episodes; audible output, network-loss recovery and Linux UI were not verified. Known prototype polish: long episode labels truncate in settings, quality selection resets to Auto on a new manifest, and the previous playback status remains after returning to results.

The isolated `desktop-rust/examples/rutube_prototype.rs` Cargo example is a local-only Rutube search/ArtPlayer experiment, not a release feature. It does not start TorrServer; its online history uses the selected local profile (use PIRATE_CINEMA_DATA_DIR for isolated testing). Only explicit searches/video selections contact Rutube. Existing pinned ArtPlayer/hls.js assets are reused. HLS uses an in-memory native Rust bridge because tested CDN replies omit browser CORS headers; HTTPS requests are limited to Rutube/rtbcdn subdomains, redirects are disabled, fragments are capped at 32 MiB with timeouts, and byte ranges are supported. Access-denied/no-HLS results fail visibly without authentication/DRM workarounds. Build with `cargo build --manifest-path desktop-rust/Cargo.toml --release --example rutube_prototype`; checks: `cargo test --manifest-path desktop-rust/Cargo.toml --example rutube_prototype` and `node desktop-rust/tests/rutube-player.cjs`. No publication or installer changes are implied.

Online history is stored separately in profile/rutube-history.sqlite, never in torrent playback tables. Per-video metadata, playlist URL, position, duration and local last-played timestamp are saved after actual playback starts, at most once per three seconds plus pause/end and an explicit flush before player destruction. Abrupt process termination can lose the last three seconds. Loaded metadata restores unfinished positions; completed videos (last five seconds) restart from zero. Recent cards on Home and the online search page deduplicate by playlist and resume its last played member, reloading the public playlist only after a click; no history is uploaded and signed HLS URLs are never persisted. Navigation during playlist restoration cancels applying the response. Rust tests reopen the real SQLite file and verify per-episode isolation, completed-video handling and invalid progress rejection; Node checks cover restore and explicit flush. Release build/clippy and existing player-state/switch/controller tests passed. Full Windows close/relaunch/resume UI verification remains pending; the attempted isolated UI test was not completed.

The same Rutube UI/transport is shared from `desktop-rust/src/rutube_prototype.rs` with the full local prototype behind the opt-in `rutube-prototype` Cargo feature. Default production builds exclude the module and navigation entry. The feature build adds a dedicated sidebar section, retains search state on navigation, stops Rutube playback on exit and pauses/hides the torrent player while the section is active. Torrent buffering/recovery starts are also gated while Rutube is active. CSS is scoped to avoid changing existing library/settings screens. Build with `cargo build --manifest-path desktop-rust/Cargo.toml --release --bin pirate-cinema --features rutube-prototype`. The local integrated executable is `local-builds/player-prototype/pirate-cinema-rutube-test.exe`; existing prototype executables/profile and released packages are not replaced.

Rutube ArtPlayer settings expose `Качество`: Auto and available HLS levels, sorted by resolution while retaining the original manifest indexes. Selection changes `hls.nextLevel` on the same HLS/player instance, preserving time, pause and fullscreen rather than resetting the media URL. The menu is rebuilt on manifest/level changes; invalid choices are ignored. Node regression checks cover Auto/manual selection, original indexes, empty lists and retained player state. This remains prototype-only.

Selecting an online video now enters a dedicated viewing layout: the source search form, pagination and results, plus the shell's torrent search/status notice, are hidden while the existing player subtree stays mounted. The viewing space contains the title, player and Back to Search action; returning stops the session but retains search results/query/page. Styling is scoped to the active viewing state and uses WebView `:has()` for the shell header. No second player or release changes are introduced.

The dedicated-view build is saved locally as `local-builds/player-prototype/pirate-cinema-rutube-player-test.exe`. Windows WebView verification confirmed no search controls/results on the viewing screen and restored the unchanged query and 100 search results through Back to Search. Existing source-player and torrent-controller regression tests passed.

Visibility switches explicitly set both `display:none` and `display:block`; clearing the style with an empty string did not restore the persistent Rutube subtree during local WebView verification. HLS requests with zero/absent byte-range end are sent as ordinary requests, not an invalid zero-length Range. The Node bridge regression covers both cases. An ignored `live_rutube_fragment` test resolves one public video and follows its manifests to download one fragment through the real Rust transport; it passed locally.

Local integrated validation used a separate empty profile and an unavailable custom endpoint to avoid starting or changing any user TorrServer. The Rutube page returned 100 live search results, loaded an open video's HLS, and played video with an advancing timer; navigating back to Library removed the Rutube player. All-target/all-feature Rust tests passed with loopback network access enabled; restricted-network execution cannot run the local HTTP fixtures. Player-state/switch/controller, diagnostics-copy and Rutube bridge Node checks also passed.

A standalone local support artifact `local-builds/Pirate-Cinema-Diagnostics.cmd` collects bounded, read-only Windows network checks: port 8091 listeners, direct versus environment-proxy curl requests to TorrServer and a fixed public Cinemeta metadata URL, and sanitized proxy status. It writes a timestamped UTF-8 report without response bodies, command lines, library data or automatic upload. This checks curl connectivity, not the Rust HTTP client's proxy behaviour; an HTTP 200 alone does not verify GST readiness. `--self-test` verifies credential redaction without network requests. The artifact and generated reports are not release payloads or committed runtime data.

Release notes for 0.7.2 are kept in `docs/RELEASE_0.7.2.md`; the tag workflow requires that file when publishing the GitHub release.

The Windows release workflow verifies and stages only the GST MatriX.144 binary; NSIS installs it without a regular-server fallback. The first successful library refresh configures HLS transcoding and checks GST `/gst/echo` even when a pre-existing TorrServer is reused, but only when current saved settings select the embedded bundled player. A GST failure appears as a player warning without incorrectly marking the server offline. Playback checks GST again before opening the embedded player. The official Windows GST binary contains its runtime; no separate system GStreamer installer is required. The release workflow runs on manual dispatch or a version tag, not on every branch push.

The `v0.5.2` and `v0.5.3` tags are intentionally published as GitHub pre-releases. Change the release workflow back to a stable/latest release before the next production tag.

Local Electron app: React/vinext renderer (3000) → Node API (3001) → TorrServer (8090), SQLite via node:sqlite and MPV over Windows named-pipe or Linux Unix-socket IPC.

- app/page.tsx: home, search and library.
- app/Playback.tsx: movie detail page with cached description and file selection, explicit resume/start, per-file history actions and MPV window controls.
- server/api.mjs: local routes, torrent sync and metadata/poster orchestration.
- server/library-sync.mjs: full-library sync, progress counters, per-title metadata lookup and cache.
- server/mpv-player.mjs: owned MPV sessions, JSON-line IPC, real-index queue and EOF-only optional auto-next.
- server/database.mjs: media and per-file history. Viewed means launched once, not completed.
- electron/main.mjs: local service lifecycle. Do not stop services not owned by the app.
- electron/preload.cjs: sandboxed bridge for the native external-player file picker.
- Tests: pnpm test. Lint: pnpm run lint. Windows build: node node_modules/vinext/dist/cli.js build. Offline installer: pnpm run desktop:package. Web installer: pnpm run desktop:package:web.

Use the project Ponytail skill at .agents/skills/ponytail/SKILL.md. Its source is DietrichGebert/ponytail, commit 2ed6c52c9d7e5e56942508591085fd45dea277d3 (MIT). It is instruction-only: no runtime dependency, hooks, telemetry or code graph.

MEX is retired. Historical notes and graph are archived under docs/archive/mex-2026-08-28 and are not active instructions. Do not run MEX or load the archive by default.

## Rust desktop

`desktop-rust/` is the production desktop application. Dioxus provides the Rust UI; TorrServer ownership, MPV IPC, SQLite history, metadata, backup/restore, updates and tray integration are implemented in Rust. The release replaces Electron program files and performs a one-time, non-destructive import of the Electron 0.5.8 profile. Windows ships one offline NSIS installer; Linux ships one x86_64 AppImage. The retired egui application is not built or packaged.

The auto-next switch is available for every multi-file torrent, regardless of its saved movie/series classification.

The file-card `Запустить` action uses a solid white treatment so it remains distinct from metadata and secondary controls.

The home page is local-first: it shows up to twelve recently launched films and series from SQLite, deduplicated by torrent, without a public-catalogue carousel or startup metadata requests. Search-result `Добавить` and `Смотреть` actions use the same solid white treatment as the file-card launch action. Cinemeta responses are parsed when a complete JSON body has arrived even if its redirected chunked catalogue response fails to close cleanly.

Cinemeta metadata, search and poster endpoints remain independent. At startup the application waits for TorrServer to return the library and then repairs only cards missing a poster or metadata in the background; concurrent metadata repairs are serialized to protect SQLite and poster files. Full metadata synchronization reports the current card count while processing large TorrServer libraries.

The UI accesses title data only through `metadata::MetadataManager`. The provider boundary uses no-key Cinemeta as the primary catalogue, TVmaze for series when Cinemeta has no match, and Wikidata by IMDb ID to locate the exact Russian Wikipedia article. A matched article supplies the Russian title and full summary while Cinemeta/TVmaze supplies the poster, rating and year; short Wikidata descriptions remain a fallback. Candidate titles are checked before accepting TVmaze/Cinemeta results, and an older Russian description is not overwritten by an English fallback. Library refresh derives compact Russian and original-language candidates from both saved and raw release names before network lookup, so manual title edits are not required for season bundles or quality-tagged releases. A failed public metadata request no longer prevents the local MPV poster-frame fallback; it is capped at six seconds per unresolved card, and public metadata/poster calls are bounded to five seconds. `live_refresh_writes_a_description_and_poster` is an ignored live test that verifies a clean temporary profile receives both fields from the public providers.

The bundled TorrServer remains running when the optional RuTor settings call fails after startup; only an actual startup or health failure marks it offline. The in-app Windows updater launches the downloaded system-wide NSIS package through an explicit UAC `runas` request instead of surfacing Windows error 740 as a launch failure.

Linux release packaging pins appimagetool 1.9.1 and verifies its published SHA-256; it does not depend on the mutable `continuous` asset.

Newly added torrents are classified locally as a movie or series from their title and real TorrServer video-file list, then immediately enter the existing metadata/poster pipeline. Mixed releases expose movie/episode grouping plus season and episode selectors; files retain their original TorrServer IDs and are ordered by group, season, episode and path.

Series navigation treats `S00`, OVA and named specials as a real Specials group instead of the all-seasons sentinel. Playback performs the existing bounded TorrServer stream probe before starting MPV, and startup repairs only library cards missing metadata or posters in a non-blocking background pass.

The series screen remembers the last season, episode and auto-next choice per torrent, displays watched/total progress for every season and keeps movies, normal episodes and specials in stable groups. MPV restores both audio and subtitle tracks. Library cards open their torrent details without an extra Continue control; resume and episode choices remain available in the detail page.

The Windows player uses the Dioxus WebView2 shell with pinned local ArtPlayer and hls.js assets plus TorrServer's GST build. ArtPlayer provides the controls and keeps compact season, episode and audio-track selectors inside the player; hls.js remains the HLS transport through ArtPlayer's `customType` hook. TorrServer remuxes or transcodes torrent media to `/gst/{hash}/master.m3u8`. Playback waits for an 18-second startup buffer and keeps up to 60 seconds ready, which avoids periodic starvation on torrents whose HLS fragments are short and uneven. Resume and audio switching keep TorrServer's complete timeline and apply the saved position only through hls.js, avoiding the previous double seek and zero-based history after changing tracks. The audio selector retries its bounded GST media probe while a fresh pipeline warms up, shows every returned track and changes the GST `audio` stream at the current position. The episode selector uses the existing naturally ordered real-file queue and returns selection to Rust so history and auto-next remain correct. Entering video fullscreen also switches the native application window to borderless fullscreen. The HLS path goes directly to GST instead of running the legacy ten-second raw `/stream` probe first; that probe remains limited to MPV and external-player launches. A five-second GST heartbeat and bounded hls.js network/media recovery keep long torrent sessions active without hiding a terminal error; a successfully loaded fragment clears a stale player error. Playback stays inside the application for MKV, AVI and other torrent containers, saves progress every five seconds and keeps auto-next. The old Win32 MPV `--wid` host was removed because WebView/native-child z-order and teardown could crash the entire UI. When a `portable-data` directory exists beside the executable, the application keeps settings, history and TorrServer data there so local testing cannot lock or overwrite an installed application's profile.

ArtPlayer is localized in Russian, uses a blue timeline and places compact season, episode and audio-track selectors in the player's upper-left corner. Starting playback or changing an episode scrolls and focuses the player automatically.

The local player-page prototype opens ArtPlayer on a dedicated viewing page. Navigating to another page only changes the same player container into a fixed mini-player; the HLS session and per-file history remain active. Expand returns to the viewing page, minimize returns to the previous page, and close ends the embedded session. This prototype also includes the Russian Wikipedia/Cinemeta/TVmaze metadata repair above; it is not a published release.

Same-torrent episode transitions now switch the source on the existing ArtPlayer and DOM container, retaining fullscreen and the event bridge across successive episodes. EOF no longer destroys the player. Progress events identify the current file; stale audio probes cannot replace the new episode's tracks. Settings includes an emergency exit for native, HTML and ArtPlayer web fullscreen.

The local player prototype has a native/window-visibility start gate: minimized or tray-hidden windows cannot start or restart playback; an EOF transition waits for restoration. Already-playing video is not forcibly paused merely because the window is minimized. Startup buffering uses the smaller of 18 seconds or the known remaining duration. Source switches use ArtPlayer's URL setter, not `switchUrl`, whose internal metadata listener resets time to zero. Pause, seek, episode selection and player/application close flush initialized progress; slow loading cannot overwrite a saved resume position with zero. Full-close flushing is bounded to 700 ms, including the no-tray fallback.

Web-player audio preferences store language/title separately from MPV numeric IDs in SQLite. Automatic language fallback never overwrites an explicitly selected studio. The player topbar includes the existing per-torrent auto-next option; its settings panel exposes HLS subtitles and local SRT/VTT (maximum 2 MiB), rendered through native video text tracks. GST enables supported text subtitles; bitmap subtitle conversion is not provided. Fatal playback errors keep an explicit Retry action; recovery reattaches HLS at the current position without replacing ArtPlayer or fullscreen. While the last two minutes have at least 30 seconds buffered, one bounded four-second probe prepares metadata for the next episodic file. This deliberately does not start a competing GST task or download the next video stream.

Run `node desktop-rust/tests/player-switch.cjs`, `node desktop-rust/tests/player-state.cjs` and `node desktop-rust/tests/player-controller.cjs` for reuse-path, decision and deterministic controller checks. The controller check covers sequential episodes, progress attribution, deferred hidden-window EOF, auto-next, recovery and retained player/fullscreen identity with simulated media boundaries; it does not replace live WebView/TorrServer playback testing.

ArtPlayer invokes its custom HLS hook asynchronously after construction. Controller initialization takes the video element directly from `art.template.$video`, rather than assuming the hook has already populated it. The regression test models this delay and verifies episode/audio controls, next-episode visibility and the fullscreen event bridge.

For an episodic file with a later episode in the same torrent, the embedded player shows a compact `Следующая серия` button when the known remaining duration reaches two minutes. Clicking it uses the existing episode-switch path and starts the next episode from the beginning; the existing optional automatic transition at EOF remains unchanged.

Release 0.6.3 fixes the file-card CSS cascade: the specific launch-button rule follows the generic file-button rule, so both its normal and hover states remain visibly white.

Release 0.6.4 no longer blocks creation of the desktop window while bundled TorrServer starts. The UI opens immediately and retries the local service in the background for up to one minute. Arch/CachyOS packaging now declares the required `webkit2gtk-4.1` runtime; DEB and RPM metadata declare their equivalent WebKitGTK runtime packages.

The release workflow compiles the application natively in a fresh Arch Linux container, rejects unresolved ELF dependencies, and requires the Dioxus process to remain alive for twenty seconds under DBus and Xvfb before any tagged release can be published. This prevents Ubuntu-linked ABI requirements such as `libxdo.so.3` from being repackaged as an Arch build where only `libxdo.so.4` is available.

Linux CI creates one x86_64 AppImage with Pirate Cinema, TorrServer and the required `libxdo.so.3` compatibility library using checksum-verified linuxdeploy and appimagetool binaries. It deliberately uses the distribution's MPV and complete GTK/WebKitGTK 4.1 stack because WebKit helper paths and GLib symbols differ between Debian and Arch. The same Arch smoke job must keep both the native Arch build and AppImage processes alive for twenty seconds. DEB, RPM, portable archives and PKGBUILD are retired from new releases.

AppImage runtime dependencies are documented on the repository landing page with copyable commands for Debian/Ubuntu, Fedora/RHEL and Arch/CachyOS. Failure to initialize the optional Linux tray must not panic the Dioxus tree; the window remains usable and closes normally when no tray provider is available.

The bundled TorrServer starts with explicit loopback IP, port 8091 and profile path arguments, so its database location no longer depends on inherited process state. Startup failures retain their concrete error in the UI, and the offline status uses a red indicator.

Windows development builds use only the local `TorrServer-gst-windows-amd64.exe` runtime beside the application or in `desktop-rust/vendor/torrserver`. The settings page stores two mutually exclusive close actions: minimize to the tray or close fully. A full exit explicitly tears down the TorrServer child owned by Pirate Cinema; the tray action remains available when minimize-to-tray is selected.

The Windows installer carries Microsoft's official x64 Visual C++ Redistributable and runs it silently before the first launch, so clean Windows Sandbox installations do not fail with a missing `VCRUNTIME140_1.dll`.

No hosting, cloud persistence or automatic publication. Do not commit databases, caches, binaries, installers or secrets. The source repository is `cyberboy1999/pirate-cinema`; publishing still requires explicit user permission.

The public README is intentionally version-agnostic: installation examples use filename wildcards and always point to the latest GitHub release. Exact version numbers belong in release notes and technical history, not on the repository landing page.

The repository landing page uses `docs/images/pirate-cinema-home.png`, a cropped main-screen capture without a version-specific native title bar.

THIRD_PARTY_NOTICES.md records the versions, licenses, source links and SHA-256
hashes of the TorrServer, MPV and FFmpeg binaries distributed in releases 0.3.2 through 0.4.0.
Keep it updated whenever a bundled binary changes.

## Playback contract

- POST /api/mpv/:hash validates the file against TorrServer. Body: fileIndex, mode (resume/start), existing (ask/replace/new), optional sessionId, autoNext (false by default).
- Same-file relaunch focuses the owned window; another file returns HTTP 409 with active windows until the user chooses replacement or a new window.
- GET /api/mpv/sessions lists owned windows. POST /api/mpv/sessions/:id supports focus, next, autoNext (boolean value) and stop.
- GET /api/torrents/:hash/files includes saved time, duration, progress and viewed status. POST .../files/:index/history accepts viewed (boolean value) or resetPosition.
- File-loaded marks Viewed once per launch. Progress updates never undo a manual unmark. The historical viewed-on-launch backfill is a one-time migration.
- Resetting position preserves Viewed and duration; close that file's MPV window first. SQLite saves about every 5 seconds and on end, switch or close.
- Queue uses natural folder/season/episode ordering and original TorrServer file IDs. Errors, manual stop and window close do not auto-advance.

## Verification

pnpm test covers SQLite migration, MPV IPC state transitions and isolated local HTTP API. Optional real-binary check: node scripts/check-mpv-ipc.mjs (generated local clip, headless bundled MPV, in-memory SQLite; no user media).

The production build and full TypeScript check use only the local Electron/vinext application. The retired Cloudflare Worker, D1/Drizzle example and hosting dependencies were removed after 0.4.4.

## Descriptions and full sync

Descriptions and metadata lookup timestamps are persisted in SQLite. GET /api/torrents/:hash/details returns cached card metadata and refreshes it when older than one day. Existing configured providers are reused; missing descriptions do not block playback.

POST /api/sync defaults to a full refresh of every saved TorrServer torrent, including previously matched metadata. Background calls pass full:false and reuse the daily cache. GET /api/library exposes syncProgress (processed, total, failed, full). Two metadata requests run concurrently. A manual full refresh waits for an ongoing background pass and then runs the full pass.

TorrServer is the source of the saved torrent list. Invalid/failed list responses cannot clear SQLite. Sync retains active MPV torrents and recently added cards during reconciliation. Local per-file playback history takes precedence over legacy TorrServer viewed data; metadata upserts preserve playback flags.

Each search release has its own Add to TorrServer button. The API immediately inserts the saved card and returns item; adding alone does not launch MPV. Opening a card displays its description and file picker together.

Metadata is provided by Cinemeta, TVmaze and Wikidata without credentials. Wikidata is queried only with public IMDb identifiers returned by the catalogue; local history and library data never leave the device.

SQLite retains description source URLs and prevents an English fallback from replacing saved Russian text. Adding the source column invalidates old metadata timestamps once (without clearing descriptions or playback history). Open a card or run full sync to refresh existing descriptions. Settings and the native window title display the version imported from package.json/Electron app metadata.

## First-run preferences

`data/config.json` stores the completed welcome state, `ru`/`en` interface
language and either bundled MPV or an explicitly selected local Windows player.
The welcome screen is shown until valid preferences are saved; the same language
and player controls remain available in Settings. Electron exposes only a native
`.exe` picker through its sandboxed preload bridge. The local API validates the
selected path before saving it.

Bundled MPV remains the default and preserves named-pipe IPC progress tracking.
An external player receives the exact selected TorrServer stream URL and marks
the file launched, but cannot report its playback position back to Pirate Cinema.

## Web installer

electron-builder-web.yml extends the offline packaging configuration and uses
the native nsis-web target. The small installer downloads the immutable x64
NSIS application package from GitHub Release v0.4.0, then installs the complete
prebuilt app and bundled MPV, TorrServer and FFmpeg under Pirate Cinema. No
compilers, Node.js or package manager are installed on the user's computer.
The offline packaging command remains unchanged.

The 0.3.3 Web Setup incorrectly used a release-directory URL and received HTTP
404 on clean machines. Version 0.3.4 embeds the complete package asset URL.
Always publish both the package and Web Setup; publish Offline Setup as the
recommended option for networks where GitHub requires a VPN.

Both NSIS installers are per-machine and request UAC before installation. The
shared `build/installer.nsh` hook closes the Pirate Cinema window, waits for its
owned local services to exit, then uses a product-name-scoped `taskkill` fallback
before replacing files in Program Files. The application itself remains
`asInvoker`; normal playback never requires administrator privileges.

## Maintenance and releases

Settings exposes system checks for the API, TorrServer, MPV, FFmpeg, selected
player and SQLite file. Each torrent file also has a playback diagnostic that
checks the exact file index before suggesting a retry. Russian and English cover
the shell, search, library, file picker, MPV controls and maintenance actions.

Electron creates timestamped backup folders containing `data`, TorrServer state
and a versioned manifest. SQLite is checkpointed before copying. Restore requires
an explicit native confirmation, replaces local state, then restarts the app.

The Rust application checks the public GitHub release feed and downloads the
single Windows NSIS installer after explicit user action. Linux AppImage users
replace the executable with the newer release after reviewing its checksum.

`.github/workflows/release.yml` runs tests and lint for version tags, restores
the pinned MPV and TorrServer payload from the public 0.3.6 bootstrap package,
verifies both SHA-256 hashes, builds the Windows installer and Linux AppImage,
and publishes only those two application artifacts plus checksums and notices.
Updating bundled runtime versions requires updating the bootstrap source and
hashes together.

The legacy suffix workflows remain only for historical release maintenance.
Normal releases use a single plain version tag. The AppImage uses system MPV,
GTK and WebKitGTK; required package-manager commands are documented in README.

## Library experience in 0.5.0

The home page shows up to six unfinished titles from SQLite before the popular catalogue. Continue Watching is based on saved playback position below 92%, independently of the launch-based Viewed marker.

The library can be filtered by media type, Viewed state, year and genre, and sorted by added date, title or year. Multi-file torrents expose episode-name search, season filtering and a shortcut to the next unviewed file.

Bundled MPV observes the active `aid` property over IPC. The selected positive audio-track ID is stored per torrent in `media_items.audio_track_id` and reused for subsequent files in that torrent.

Electron owns a native system tray. Closing the main window hides it without stopping owned local services; double-click or the Open menu restores it, and only the tray Exit action or application shutdown terminates the children.
## Playback choice layout in 0.5.1

The normal file-launch confirmation uses one compact action row. Resume remains available when progress exists, the start-from-zero action is labelled Play, and Back to files stays in the same row. The existing-player conflict choices retain their vertical layout.

## Home and magnet additions in 0.5.2

The home screen uses the selected cinematic direction: one large Continue Watching item, followed by a compact six-title popular shelf. The shell uses a narrower rail, compact search, inline TorrServer state, one sans-serif family and monochrome tokens; green is reserved for online and success states.

POST `/api/torrents/add` now waits for the existing metadata enrichment path before returning. A magnet `dn` value is parsed into a clean title and year immediately, and configured metadata providers can return the poster and description in the same response instead of waiting for the next library sync.

## Metadata correction and home catalogue in 0.5.3

Each library card has a compact title editor. The saved `metadata_query` survives TorrServer reconciliation and is used for forced public-provider enrichment, so an incorrect release name no longer makes every full sync repeat the same failed lookup. Editing clears stale matched metadata before the fresh lookup.

The local follow-up build replaces Electron's unreliable `window.prompt` editor with an in-app modal. It keeps the form open during lookup, shows validation/provider errors inline and closes only after the refreshed library has loaded.

Poster matching also reuses the release year when a manual title omits it, preventing ambiguous remakes such as `Shogun` from selecting an older version. When a catalogue item has no usable cover, its IMDb ID is used for a bounded Wikidata artwork lookup; a local frame remains the final fallback.

The home shelf loads up to 30 current movies from Cinemeta's public top catalogue and falls back to the bundled list when the source is unavailable. The generic catalogue request contains no library data. Shelves longer than six cards scroll horizontally with accessible previous/next controls.

## Optional Torznab search in the local follow-up

Settings accepts one optional Prowlarr or Jackett Torznab URL and API key. Pirate Cinema applies it through TorrServer's settings API without bundling or starting another executable, masks the saved key in its own API responses, and merges deduplicated Torznab results with RuTor search. Clearing the integration disables Torznab without affecting the external indexer.

## Jackett labelling in 0.5.5

Search results received from a Jackett aggregate Torznab URL are labelled `Jackett` plus the originating indexer name. Other Torznab-compatible services retain the generic `Torznab` label. Release 0.5.5 is stable and keeps bilingual release notes and documentation.

Torznab sources are configured with `CatType: all`; TorrServer's default movie/TV category filter can otherwise hide valid Jackett results when an indexer uses different category mappings.

Version 0.5.6 also accepts HTTP(S) torrent download links returned by Torznab providers, while continuing to reject unsupported URL schemes.

## Next episode and duplicate protection

For a continuing series, the home hero fetches the existing sorted file list and names the first unviewed episode; movies never trigger episode lookup. Adding a different info hash with the same normalized title and compatible year requires explicit confirmation, while re-adding the exact same hash reuses the saved TorrServer item without a duplicate prompt.

The empty-library state must keep `nextResult` nullable. The home overview only reads its file after confirming that a result exists and belongs to the featured torrent; this prevents a clean installation from failing its initial render.
