# Pirate Cinema

Pirate Cinema is a local Electron media library that searches and streams torrents through TorrServer and plays them in bundled MPV.

## Rules
- Keep the application local-first; do not add hosting or cloud persistence.
- Do not transmit library data unless the user explicitly authorizes the provider.
- Keep TorrServer on the configurable local endpoint (default `http://127.0.0.1:8090`).
- Preserve MPV IPC progress tracking and per-file SQLite history.
- Never commit secrets, API keys, runtime databases, caches, or generated installers.

## Commands
- Development: `pnpm run dev:local`
- API only: `pnpm run api:local`
- Tests: `pnpm test`
- Lint: `pnpm run lint`
- Web build: `pnpm run build`
- Windows package: `pnpm run desktop:package`

## Ponytail and project context
Use the project skill `.agents/skills/ponytail/SKILL.md` for coding tasks. Read `docs/PROJECT.md`, then inspect the actual affected code and callers. Reuse existing code and standard libraries; keep validation, error handling, accessibility and tests.
After meaningful changes, update `docs/PROJECT.md` and run the relevant tests. MEX is retired; do not run it or load its archived graph by default.
