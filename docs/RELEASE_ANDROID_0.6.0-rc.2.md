# Pirate Cinema Android 0.6.0-rc.2

This prerelease adds local RuTor torrent discovery to the embedded librqbit Android client.

## Changes

- Searches the official TorrServer RuTor catalogue locally without sending the query or media library.
- Keeps the downloaded catalogue in the Android app cache and safely replaces it only after a successful parse.
- Uses the existing native Rust library for bounded streaming search and ranking.
- Shows quality, year, size and seed count with separate **Add** and **Watch** actions.
- Rejects malformed queries and bounds compressed input, decompressed JSON, strings, alternate names and retained results.

## Verification

- Android ARM64 release build, signing certificate and 16 KiB alignment checks.
- Android emulator instrumentation test for raw-DEFLATE parsing, nullable fields, ranking and invalid-query rejection.
- Real emulator search returned 40 bounded results for `Матрица`; the final cached-index run completed in 13.7 seconds under ARM64-to-x86_64 translation.
- Mobile TypeScript, ESLint and 16 targeted Node tests passed.

The APK is ARM64-v8a and remains a release candidate.
