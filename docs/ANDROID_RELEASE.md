# Android release gate

The candidate is **0.6.0-rc.1**, versionCode **61**, Android 7+ / arm64-v8a.
`android/version.properties` is the single version source for Gradle and the UI.
Do not mark the candidate stable until the device/upgrade/long-playback gates pass.

## Signing

`scripts/build-android-release.ps1` checks mobile TypeScript, builds/syncs the UI, runs mobile tests, Android release lint, signs the APK, verifies its pinned certificate, production application ID and 16 KiB ZIP/ELF alignment, then writes APK and SHA256SUMS under ignored `release-local/android/<version>/`. Rust dependency licenses are collected from the locked local crates and bundled alongside component license texts.

The permanent key and DPAPI-encrypted credential are outside Git under `%LOCALAPPDATA%/PirateCinema/android-signing/`. Never commit/upload that directory. Back up the PKCS12 file and its password to a private encrypted vault: the DPAPI credential alone cannot be decrypted after losing the Windows account/machine. Keep this signing identity for every future update.

To move the password into your own password manager, run this locally (it copies to the clipboard, not the terminal or this chat), paste into the private vault, and clear the clipboard afterward:

```powershell
(Import-Clixml "$env:LOCALAPPDATA/PirateCinema/android-signing/signing-credential.xml").GetNetworkCredential().Password | Set-Clipboard
# After saving in your private vault:
Set-Clipboard -Value ''
```

Also back up `pirate-cinema-release.p12`; neither the public certificate fingerprint nor the password alone can replace the private key. No signing material is attached to GitHub releases.

Other build hosts may use the four `PIRATE_ANDROID_*` signing environment variables named in `android/app/build.gradle`; no passwords belong in Gradle properties or source control. A release without them fails instead of silently using a debug key.

## Device checks

- Preserve the existing app profile: use `adb install -r` for the same signing identity.
- Do not install release over a debug-signed app or uninstall a user profile to resolve a signature conflict. Export backups first and use an isolated QA package/device for release smoke tests.
- Do not use Gradle `connectedAndroidTest` on a valuable profile: it may uninstall the application. Build the test APK, install explicitly and use `am instrument` instead.
- Verify RuTube search/playlist/resume, torrent range/seek/audio/quality/episode selection, pause/restart, background/foreground, force-stop/reopen, low-space/quota protection and backup rollback.
- Verify 20 minutes of real playback, not merely assigned currentTime or a synthetic ended event.

The engine owns a foreground data-sync service with an explicit stop action, stops when its task is removed and never auto-starts on boot. Android may time-limit data-sync foreground services (the timeout shuts the engine down). Storage is app-private: default soft torrent-data limit 8 GiB, adjustable 1–128 GiB; reserve 512 MiB, sampled every five seconds. Crossing a limit pauses downloads, does not silently delete files, and requires manual resume after freeing space/changing the limit. Removal explicitly asks whether downloaded files should also be deleted. HLS segment cache is separately bounded.

Backups include playback history, metadata, RuTube videos and loaded playlist queues, not torrent payload files or private tracker credentials. Restore is validated and quota-failure rollback is tested; the previous snapshot is available with Undo last restore. Cinemeta title lookup requires explicit consent. Android cloud backup and device transfer are disabled; export JSON through the local document picker.

## Publication

Publish only the Android artifact, checksums, release notes, third-party notices and matching source/build instructions. Use an `android-v<version>` tag: desktop `v*` workflows must not be triggered. The release must point to the actual reviewed source commit, not an older HEAD with an APK built from uncommitted changes. No secrets, databases, fixture movies, native build caches or installers in Git.
