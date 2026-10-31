---
version: 1
slug: "desktop-rust-src-bin-dioxus-rs"
primary_target: "desktop-rust/src/bin/dioxus.rs"
related_targets: []
---

# App shell

## Scope

Whole Dioxus desktop application: first run, Home, Search, Library, Favorites, Watch later, History, Detail, Player chrome, Settings, remote overlay and global navigation.

Mode: Operate. The visitor is managing and watching a personal library; familiarity, scanability, playback safety and fast recovery outrank spectacle.

## Audience and job

A desktop user with an existing local library wants to move from opening the app to resuming or starting a specific title with as little navigation as possible. The same surface must remain comfortable from a sofa with a keyboard, gamepad or remote, and from a desk with a mouse.

## Constraints

- Preserve the Rust/Dioxus, embedded librqbit, ArtPlayer/hls.js and SQLite architecture.
- Preserve all current playback, search, metadata, history, remote, backup and update behaviour.
- Keep new organisation data local and migration-safe.
- Keep `Pirate Cinema 0.8.0` as the window title.
- Use real library and poster data; no fabricated titles or claims.

## Direction contract

THESIS: A private screening room that puts the next film before the machinery, refusing the category-default dashboard of equal-weight cards and utility panels. The shell is quiet; the artwork, title and resume action carry the first moment.

OWN-WORLD: Warm-black cinema ground (#08090c), charcoal glass surfaces, ivory type, muted warm-grey secondary text, and a single amber cue light (#f5c451) for focus, progress and saved state. Rounded poster planes, hairline dividers, blurred artwork fields, a fixed icon rail and one wide shelf rhythm make the surface recognisable with all content removed. Primary actions are ivory pills; secondary actions are translucent charcoal pills.

STORY: The visitor understands at a glance what they were watching, what is new, and where their saved films live. They can resume or open a title, search a release, organise it into Favorites or Watch later, and trust that history and playback remain local.

FIRST VIEWPORT: A slim black navigation rail at left with the brand mark and five destinations. The content begins with a top bar: back affordance, a wide pill search field, engine status and settings. Home opens on a 16:9 spotlight stage built from the featured title's blurred artwork, a large poster plane at right, title and metadata at left, and Resume / Details actions. Immediately below, a Continue watching rail of landscape progress cards begins before the fold; the next rail is New in the library. Nothing in the first viewport is a generic dashboard tile.

FORM: The direction is pinned by the brief: Apple TV's dark, artwork-led calm crossed with Plex's library precision. The ordered concept roll did not run; `seed: none (unavailable)`.

SIGNATURE: A warm amber focus cue travels across dark rails and poster planes like a cinema aisle light; the spotlight's poster plane shifts subtly on pointer movement while reduced-motion users get the static composition.

FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance.
