# Design QA — pre-release 0.5.2

## Reference and test state

- Source: `C:\Users\savay\.codex\generated_images\01a09efe-c790-7801-aec3-470a84f58113\exec-90cc6a0e-290a-48ca-a3a5-d9669d598463.png` (selected option 2, 1487×1058).
- Implementation capture: `artifacts/qa-0.5.2-home-final.png` (1440×1024, device scale factor 1).
- Side-by-side evidence: `artifacts/qa-0.5.2-comparison-final.jpg`; both views normalized to 1440×1024 before stacking.
- State: Russian locale, TorrServer online, “Сёгун” at 19% in Continue Watching, six popular titles.

## Findings and fixes

The first implementation pass had two P2 mismatches: the portrait artwork was enlarged into a distracting crop, and the search control was visibly shorter than the reference. The hero artwork is now contained on the right with a dark gradient, and the search width is aligned with the reference hierarchy.

The final full-view comparison has no open P0, P1, or P2 issues. The shell, spacing, type hierarchy, compact server state, single primary action, 2:3 shelf, and restrained three-level palette match the selected direction. A P3 content difference remains intentionally: the local media schema currently stores poster art but no landscape backdrop, so the real library poster is used in the hero instead of introducing a schema/provider expansion for this small redesign.

## Functional checks

- Search field accepts and clears text.
- Main navigation buttons render and expose accessible button names.
- Viewport has no horizontal overflow at 1440×1024.
- No page errors or failed network requests were recorded on the final capture. A transient development-console 404 was rechecked and did not reproduce as an HTTP response or application request failure.

## Result

passed

## 0.5.3 extension

- The Cinemeta shelf keeps six cards visible at 1440×1024 and scrolls horizontally when more results are available.
- The next control moved the shelf by 1232 px in browser QA; previous/next controls have localized accessible names.
- Every library card exposes a keyboard-accessible metadata edit action, visually hidden until hover or focus on desktop.
- No page errors were recorded. Result: `passed`.
- The follow-up local installer replaces the prompt with a focusable in-app dialog that exposes its lookup progress and errors. Result: `passed`.
