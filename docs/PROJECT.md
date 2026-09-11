# Pirate Cinema — project context

Release version: 0.3.2. Windows installer: release/Pirate-Cinema-Setup-0.3.2.exe.

Local Electron app: React/vinext renderer (3000) → Node API (3001) → TorrServer (8090), SQLite via node:sqlite and bundled MPV over Windows named-pipe IPC.

- app/page.tsx: home, search and library.
- app/Playback.tsx: movie detail page with cached description and file selection, explicit resume/start, per-file history actions and MPV window controls.
- server/api.mjs: local routes, torrent sync and metadata/poster orchestration.
- server/library-sync.mjs: full-library sync, progress counters, per-title metadata lookup and cache.
- server/mpv-player.mjs: owned MPV sessions, JSON-line IPC, real-index queue and EOF-only optional auto-next.
- server/database.mjs: media and per-file history. Viewed means launched once, not completed.
- electron/main.mjs: local service lifecycle. Do not stop services not owned by the app.
- Tests: pnpm test. Lint: pnpm run lint. Windows build: node node_modules/vinext/dist/cli.js build. Installer: pnpm run desktop:package.

Use the project Ponytail skill at .agents/skills/ponytail/SKILL.md. Its source is DietrichGebert/ponytail, commit 2ed6c52c9d7e5e56942508591085fd45dea277d3 (MIT). It is instruction-only: no runtime dependency, hooks, telemetry or code graph.

MEX is retired. Historical notes and graph are archived under docs/archive/mex-2026-08-28 and are not active instructions. Do not run MEX or load the archive by default.

No hosting, cloud persistence or automatic publication. Do not commit databases, caches, binaries, installers or secrets. The source repository is `cyberboy1999/pirate-cinema`; publishing still requires explicit user permission.

THIRD_PARTY_NOTICES.md records the versions, licenses, source links and SHA-256
hashes of the TorrServer, MPV and FFmpeg binaries distributed in release 0.3.2.
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

The production build passes. A full tsc --noEmit currently also scans legacy Cloudflare files in db/ and worker/ and fails on their missing Worker types; the local application build does not use them.

## Descriptions and full sync

Descriptions and metadata lookup timestamps are persisted in SQLite. GET /api/torrents/:hash/details returns cached card metadata and refreshes it when older than one day. Existing configured providers are reused; missing descriptions do not block playback.

POST /api/sync defaults to a full refresh of every saved TorrServer torrent, including previously matched metadata. Background calls pass full:false and reuse the daily cache. GET /api/library exposes syncProgress (processed, total, failed, full). Two metadata requests run concurrently. A manual full refresh waits for an ongoing background pass and then runs the full pass.

TorrServer is the source of the saved torrent list. Invalid/failed list responses cannot clear SQLite. Sync retains active MPV torrents and recently added cards during reconciliation. Local per-file playback history takes precedence over legacy TorrServer viewed data; metadata upserts preserve playback flags.

Each search release has its own Add to TorrServer button. The API immediately inserts the saved card and returns item; adding alone does not launch MPV. Opening a card displays its description and file picker together.

Version 0.3.2 prioritizes Russian Wikipedia descriptions independently of Cinemeta/TVmaze posters. server/wikipedia.mjs searches localized/original names, checks media type and film year, and follows English-to-Russian language links when necessary. No new dependencies or API keys. Wikipedia credit links are shown with descriptions.

SQLite retains description source URLs and prevents an English fallback from replacing saved Russian text. Adding the source column invalidates old metadata timestamps once (without clearing descriptions or playback history). Open a card or run full sync to refresh existing descriptions. Settings displays the version imported from package.json.
