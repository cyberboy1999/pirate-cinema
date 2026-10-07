# Pirate Cinema

Pirate Cinema is a local-first desktop media library. The production application is Rust/Dioxus; Electron sources remain only for migration and compatibility reference.

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

## Development workflow
Before a substantive task, read this file, `docs/PROJECT.md`, and the durable facts in `docs/PROJECT_MEMORY.md`; then inspect the current code and its callers. Priority is: current user request, these project rules, current source/configuration, then project memory.

Use the available Superpowers skills when their process is useful: understand and research first; plan, implement, test, debug systematically on failures, and verify before completion. Do not invoke skills mechanically for trivial work.

Use the project Ponytail skill `.agents/skills/ponytail/SKILL.md` as the final complexity gate: reuse existing code and standard libraries, reject unjustified abstractions and dependencies, but do not remove necessary correctness, validation, accessibility, or error handling.

After meaningful changes, run the relevant checks, update `docs/PROJECT.md` when project documentation changes, and update `docs/PROJECT_MEMORY.md` only for durable, confirmed knowledge that is not already obvious from the source. Do not store session notes, credentials, caches, or debugging noise in Git. MEX is retired; do not run it or load its archived graph by default.
