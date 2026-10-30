# Pirate Cinema

Pirate Cinema is a local-first desktop media library. The production application
is Rust/Dioxus in `desktop-rust/`. The Electron/React sources under `app/`,
`electron/` and `server/` remain only for migration and historical reference;
they are not part of the 0.8.0 release and must not be revived as the desktop
runtime.

## Rules

- Keep the application local-first; do not add hosting or cloud persistence.
- Do not transmit library data unless the user explicitly authorizes the provider.
- The torrent engine is embedded `librqbit`; do not add TorrServer.
- Playback is ArtPlayer/hls.js over the local FFmpeg/HLS bridge; do not add MPV or Electron playback.
- Preserve SQLite per-file history, local poster cache, duplicate checks and migration safety.
- Never commit secrets, API keys, runtime databases, caches, profiles, build output, installers or generated assets.
- Keep the release flow tag-first: bump the version, commit, tag `v<version>`, and let one tag-triggered workflow build and publish. The tag must match `desktop-rust/Cargo.toml`.
- Keep `docs/RELEASE_<version>.md` bilingual (Russian and English) and publish it with the matching GitHub release.
- Never commit the Impeccable engine under `.agents/skills/impeccable/scripts/bin/` or generated `.impeccable` runtime state.

## Commands

- Format: `cargo fmt --manifest-path desktop-rust/Cargo.toml`
- Check: `cargo check --locked --offline --manifest-path desktop-rust/Cargo.toml --all-targets --all-features`
- Test: `cargo test --locked --offline --manifest-path desktop-rust/Cargo.toml --all-targets --all-features`
- JavaScript controllers: `node desktop-rust/tests/player-state.cjs`, `player-switch.cjs`, `player-controller.cjs`, `remote-controller.cjs`, `diagnostics-copy.cjs`, `rutube-player.cjs`
- Run: `cargo run --locked --offline --manifest-path desktop-rust/Cargo.toml --bin pirate-cinema`
- Release: `.github/workflows/release.yml` on a `v*` tag (or manual dispatch). It runs quality checks, Windows NSIS, Linux AppImage and the Arch smoke test.

## Development workflow

Before a substantive task, read this file, `docs/PROJECT.md`, and the current
source and callers. Priority is: current user request, these project rules,
current source/configuration, then historical project memory.

Use the project Ponytail skill at `.agents/skills/ponytail/SKILL.md` as the
final complexity gate: reuse existing code and standard libraries, reject
unjustified abstractions and dependencies, but do not remove necessary
correctness, validation, accessibility or error handling.

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

After meaningful changes, run the relevant checks, update `docs/PROJECT.md`
when project documentation changes, and record only durable, confirmed
knowledge. Do not store session notes, credentials, caches or debugging noise
in Git.
