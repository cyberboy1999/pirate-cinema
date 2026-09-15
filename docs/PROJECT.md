# Pirate Cinema — project context

Latest stable release: 0.5.7. Releases contain Windows offline/web installers, Debian/Ubuntu DEB, RPM, Arch Linux tarball and PKGBUILD. Fresh installs package no TorrServer `config.db` or `viewed.json`; SQLite and TorrServer user state are created empty on first launch.

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

No hosting, cloud persistence or automatic publication. Do not commit databases, caches, binaries, installers or secrets. The source repository is `cyberboy1999/pirate-cinema`; publishing still requires explicit user permission.

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

Version 0.3.2 prioritizes Russian Wikipedia descriptions independently of Cinemeta/TVmaze posters. server/wikipedia.mjs searches localized/original names, checks media type and film year, and follows English-to-Russian language links when necessary. No new dependencies or API keys. Wikipedia credit links are shown with descriptions.

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

On Windows, `electron-updater` checks the public GitHub release feed after startup, downloads
updates in the background and waits for explicit restart or normal app exit to
install. DEB/RPM installations are updated through the distribution package
manager or a newer release package. The offline NSIS build publishes `latest.yml`
and its blockmap.

`.github/workflows/release.yml` runs tests and lint for version tags, restores
the pinned MPV and TorrServer payload from the public 0.3.6 bootstrap package,
verifies both SHA-256 hashes, builds both installer variants and publishes release
assets. Updating bundled runtime versions requires updating the bootstrap source
and hashes together.

The legacy suffix workflows remain only for historical release maintenance.
Normal releases use a single plain version tag. Linux packages depend on system
MPV and FFmpeg; Arch uses the generated PKGBUILD and installs under
`/opt/pirate-cinema`. The shared install script verifies SHA-256 and creates
freedesktop launchers for KDE, GNOME and XFCE.

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

Each library card has a compact title editor. The saved `metadata_query` survives TorrServer reconciliation and is used for forced Wikipedia/Cinemeta enrichment, so an incorrect release name no longer makes every full sync repeat the same failed lookup. Editing clears stale matched metadata before the fresh lookup.

The local follow-up build replaces Electron's unreliable `window.prompt` editor with an in-app modal. It keeps the form open during lookup, shows validation/provider errors inline and closes only after the refreshed library has loaded.

Poster matching also reuses the release year when a manual title omits it, preventing ambiguous remakes such as `Shogun` from selecting an older version. When Russian Wikipedia identifies a series but has no image, its English title is used for a bounded second Cinemeta lookup; the Cinemeta poster is merged with the Russian description even when a season year differs from the show's premiere year.

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
