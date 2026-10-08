# Pirate Cinema — project context

## Development discipline

Codex task lifecycle is: understand the request and project rules; recall
`PROJECT_MEMORY.md`; inspect current code; apply the relevant engineering
workflow; implement; test and verify; use Ponytail to remove unjustified
complexity; then record only durable, confirmed new knowledge. Current source
and project rules override memory. The repository keeps shared project memory;
Codex session context remains local and is not committed.

Latest published release: 0.7.3; 0.7.4 is being prepared. Releases contain one Windows NSIS installer and one Linux x86_64 AppImage. Fresh installs package no TorrServer `config.db` or `viewed.json`; SQLite and TorrServer user state are created empty on first launch.

Release notes for 0.7.2 are kept in `docs/RELEASE_0.7.2.md`; the tag workflow requires that file when publishing the GitHub release.

The Windows release workflow now verifies and stages both the ordinary and GST MatriX.144 binaries; NSIS installs both and the application prefers GST. The first successful library refresh configures HLS transcoding and checks GST `/gst/echo` even when a pre-existing TorrServer is reused, but only when current saved settings select the embedded bundled player. A GST failure appears as a player warning without incorrectly marking the server offline. Playback checks GST again before opening the embedded player. The official Windows GST binary contains its runtime; no separate system GStreamer installer is required. The release workflow runs on manual dispatch or a version tag, not on every branch push.

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

The bundled TorrServer starts with explicit loopback IP, port 8090 and profile path arguments, so its database location no longer depends on inherited process state. Startup failures retain their concrete error in the UI, and the offline status uses a red indicator.

Windows development builds prefer the local `TorrServer-gst-windows-amd64.exe` runtime when it is present beside the application (or in `desktop-rust/vendor/torrserver`), and fall back to the standard TorrServer binary when it is not. The settings page stores two mutually exclusive close actions: minimize to the tray or close fully. A full exit explicitly tears down the TorrServer child owned by Pirate Cinema; the tray action remains available when minimize-to-tray is selected.

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
