# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Users

People who keep a personal video library on a Windows or Linux desktop and want to find, organise and watch it without handing their library or viewing history to a remote service. They use the app in a living room or at a desk, often with a keyboard, mouse, remote or gamepad, and expect media apps to feel immediate and legible.

## Product Purpose

Pirate Cinema is a local-first desktop cinema: an embedded torrent engine, metadata and poster cache, per-file playback history, and an in-app ArtPlayer/hls.js player in one application. Success means a person can add a release, understand what is in the library, resume the right file, and control playback without leaving the app or depending on a second local server.

## Positioning

The complete cinema pipeline is embedded in one Rust process. There is no TorrServer or MPV dependency, no cloud library, and no Electron shell; the torrent engine, local HLS bridge, SQLite history and player state live together on the machine.

## Operating Context

- The app runs as a Dioxus desktop window on Windows and Linux, with an embedded librqbit engine and loopback HTTP/HLS playback.
- Metadata and posters are cached locally; the library remains useful offline after metadata has been fetched.
- Viewing position, watched state, audio/subtitle preferences and playback preferences are stored per file in SQLite.
- The optional LAN web remote controls only the active player, not the library or search.
- The application must keep running and playable when the window is restored after being minimised; closing may minimise to the tray.
- Releases are built from pinned dependencies; there is no runtime network requirement for the interface itself.

## Capabilities and Constraints

- Embedded torrent engine and local playback through ArtPlayer + hls.js.
- Local library with search, type, genre, year, watched-state filters and metadata refresh.
- Torznab/Jackett search configured by the user; magnet links can be added directly.
- Per-file history, resume positions, watched state, next-episode flow and audio/subtitle preferences.
- Local poster cache and migration of legacy Electron profiles without overwriting newer Rust data.
- Optional LAN web remote with QR code.
- Diagnostics, local backup and GitHub release update checks.
- Preserve the release tag and the published 0.8.0 notes. Do not introduce TorrServer, MPV, Electron, cloud accounts or telemetry.
- New organisation features must be migration-safe and remain local to SQLite.

## Brand Commitments

- Keep the product name and window title `Pirate Cinema 0.8.1`.
- Keep the project-local Impeccable and frontend-design skills available as documented.
- The user has pinned the visual direction to an Apple TV + Plex-style cinematic shell: dark, poster-led, calm, premium and remote-friendly. This is a binding visual constraint, not a claim about either brand.

## Evidence on Hand

- Real local poster, metadata and playback data are available through the application's own SQLite store; empty states must be honest when no library exists.
- `public/favicon.png` is the existing brand mark.
- The current Dioxus shell and ArtPlayer integration are the behavioural source of truth.
- No customer logos, testimonials, pricing or commercial claims exist and none may be invented.

## Product Principles

1. Make the library feel like a private screening room, not a control panel.
2. Keep the next meaningful action obvious: resume, play, search or organise.
3. Stay local, fast and legible; never trade working playback for decoration.
4. Let the artwork lead while keeping titles, progress and status readable without it.
5. Support focused, remote-style navigation as a first-class way to move through the app.

## Accessibility & Inclusion

Keyboard and gamepad focus must be visible and predictable. Text and controls must meet WCAG AA contrast, interactive targets must remain large enough for living-room use, and no state may rely on colour alone. Subtitles remain available through the player.
