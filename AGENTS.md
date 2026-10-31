# Pirate Cinema Desktop 0.8.1

- Read `README.md` and `COMPONENTS.md` before editing.
- Keep the application local-first; never transmit the library or history.
- The torrent engine is embedded `librqbit`; do not add TorrServer.
- Playback is ArtPlayer/hls.js over the local FFmpeg/HLS bridge; do not add MPV or Electron.
- Preserve SQLite per-file history, local poster cache, duplicate checks and migration safety.
- Do not commit runtime databases, caches, profiles, build output, installers or secrets.
- Run `cargo fmt`, `cargo test --locked --offline` and relevant JavaScript controller tests after changes.

## Design skills

Project-local skills live in `.agents/skills/` and are discovered by OpenCode
and Codex:

- `impeccable` (`.agents/skills/impeccable/SKILL.md`) — design direction,
  critique, audit, polish, motion, typography, accessibility, responsive and
  edge-case passes. Source: `pbakaus/impeccable` 4.8.0, commit `df3a4ab`,
  Apache-2.0. The first run downloads and verifies the engine into
  `~/.impeccable/bin`; never commit `scripts/bin/` or `.impeccable` runtime
  state.
- `frontend-design` (`.agents/skills/frontend-design/SKILL.md`) — distinctive
  production-grade frontend/UI work and browser-based visual QA. Source:
  `dobromirdikov/codex-frontend-design-skill`, commit `9831197`,
  Apache-2.0, derived from Anthropic's `frontend-design`.

Invoke either through the native `skill` tool, or use `/impeccable` in
OpenCode. Use `impeccable` to review or refine the Dioxus/ArtPlayer UI; use
`frontend-design` when the task is primarily visual direction or a new
frontend surface. Keep the app local-first: project telemetry is disabled in
`.impeccable/config.json`.
