# Impeccable upstream

- Source: https://github.com/pbakaus/impeccable
- Commit: `df3a4abe7d0840d5f60324812c432e00b66a4e75` (2026-10-30)
- Skill version: 4.8.0
- Engine version: 0.1.18 (downloaded on first run; not committed)
- License: Apache-2.0. See `LICENSE` and `NOTICE.md`.

This is the portable `.agents/skills/impeccable` payload from upstream. The
provider-generated trees, hooks and engine binaries are intentionally not
committed.

## First run

On Windows run `scripts/impeccable.cmd`; on POSIX run `scripts/impeccable`.
The launcher verifies the pinned engine against `scripts/engine-checksums.txt`
and caches it in `~/.impeccable/bin/<version>/`. No Node runtime is required.
Nothing under `scripts/bin/` is part of the repository.

## Local integration

- OpenCode and Codex both discover this skill through `.agents/skills/`.
- `.opencode/commands/impeccable.md` provides the OpenCode `/impeccable`
  command; the skill itself is also available through the native `skill` tool.
- Project telemetry is disabled in `.impeccable/config.json`
  (`"telemetry": false`). The optional design-catalog request still happens
  when a new design direction is rolled; it never sends the library, history,
  local files or `PRODUCT.md`.
