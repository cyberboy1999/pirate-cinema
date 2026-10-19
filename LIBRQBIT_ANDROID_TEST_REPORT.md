# Android test report — 2026-10-18

Status: signed release candidate; stable-release criteria still require a broader device/codec matrix.

## Signed Android RC hardening

Candidate version 0.6.0-rc.1/code 61 has a permanent release identity stored outside the repository. Production application ID, certificate SHA-256, APK v2 signature, ZIP 16 KiB alignment and every bundled native ELF LOAD alignment (0x4000 for all 11 libraries) are checked by scripts/build-android-release.ps1. Building without signing credentials was verified to fail. Release Lint passes; mobile scoped TypeScript, ESLint and 16 mobile tests pass. Full repository tests: 59/61, two existing desktop packaging/workflow assertions fail.

New device checks use separately installed release-signed com.piratecinema.app.qa, never replacing or uninstalling the original debug profile. The owned 1205-second/19,338,809-byte MP4 is shared through USB with a bounded local seeder. Instrumentation restarts the process, so scripts/run-android-soak.ps1 re-adds the private USB test peer; this is test transport, not production tracker discovery. An early run lost its WebView during device interactions, and another timed out before restoring the private peer. Neither failed run counts as a passed soak. The final signed-QA run passed 20 minutes of actual playback (`OK (1 test)`, runner time 1204.864 seconds): time and decoded frames continued through the last full sample after 1170 seconds, playback remained unpaused and peak PSS was 367,065 KiB (about 358 MiB).

Foreground-service startup was observed on physical Android 16 (isForeground=true/type=dataSync). The storage guard reports allocated blocks, not sparse-file logical lengths; default limit 8 GiB and reserve 512 MiB. A separate signed-QA run passed all six instrumentation tests in 7.523 seconds: native FFmpeg/HLS checks, notification Stop followed by in-app Restart, background operation, and a real 1 GiB quota crossing while the first torrent was already paused. The allocation fixture was deleted and the previous quota restored in test cleanup.

## Latest Artplayer and streaming results

The current APK replaces default external playback with Artplayer 5.4.0, HLS.js 1.7.3 and artplayer-plugin-hls-control 1.1.0. Explicit system-player fallback remains available. Torrent playback now uses local FFmpegKit min-gpl 8.1.7 to produce requested six-second H.264/stereo AAC HLS segments from librqbit ranges. Quality/audio menus select real local variants; file/episode selection uses update_only_files and retains per-file history. No cloud transcoding is used. This is compatibility output, not original HDR/surround fidelity; broad AC3/DTS/HEVC coverage remains untested.

Final physical multi-audio/episode scenario (`node scripts/test-android-artplayer.mjs`), passed:

- Owned 60-second 640×360 MKV with two AAC sine tracks, copied into S01E01.mkv and S01E02.mkv, USB-only local seed. Hash 3e113943a038eda5bcdd65d2429c9ff969a64b0e.
- Initial decoded MKV frame, readyState 4, paused=false.
- First decoded audio peak ~439.45 Hz; after selecting second track ~878.91 Hz, verified with Web Audio analyser, not only menu text.
- 360p → 240p produces actual 240-high frames with continued playback.
- Seek to 42 seconds resumes frames at 43.130667, readyState 4.
- Switching to episode 2 produces frames at 0.072022, readyState 4; episode 1 retains its separate saved position.
- Switching back restores episode 1 to 43.196117 seconds, readyState 4, paused=false.
- Three Android instrumentation tests passed: package context, actual FFmpeg native runtime/dependencies, HLS audio variants and VOD timeline. Run with explicit install -r plus am instrument. The initial Gradle connectedDebugAndroidTest run automatically removed the debug application/test profile; the owned-series fixture was subsequently recreated. Avoid that Gradle task against a profile that must be retained.
- Targeted ESLint, mobile build/sync, native APK build, 13 mobile Node tests and git diff --check passed. Full repository suite has not passed.
- Reviewer verified fixes for cancellable probing, unique track names, atomic segment publication and stale session cache cleanup. These are code-reviewed, not fault-injection stress-tested.

Earlier single-file Artplayer checks (before the native HLS layer):

Physical phone checks executed with the current Artplayer work:

- Generated owned 1205-second H.264 testsrc2 MP4, 19,338,809 bytes, with local Rust fixture seeder restricted to 127.0.0.1:4242 and 64 KiB/s upload. Phone connected through adb reverse, not a public tracker.
- Streaming script metadata response 61 ms. HEAD 200, GET 200, five 206 exact-byte comparisons at offsets 0, 9,669,404, 19,334,713, 131,072 and 65,536; invalid range 416; concurrent requests; cancellation: passed in 33,302 ms.
- At end of those range tests only 2,233,913 of 19,338,809 bytes were downloaded; finished=false, one live peer. Far-end bytes were obtained without downloading the complete file.
- Artplayer torrent playback: currentTime 5.03234, duration 1205, videoWidth 320, paused=false. During later seek checks the torrent still had only about 2 MiB downloaded. Seek assignments to 600 and 30 seconds reached readyState 1 while buffering; resumed frames after these seeks were NOT confirmed, so seek playback is not marked passed.
- RuTube Artplayer HLS: actual currentTime 142.915611, frame 640×480, readyState 4, paused=false. The quality menu contained 480P/360P/240P/144P/Auto. Selecting 240P produced frame 320×240, readyState 4, currentTime 175.872866, paused=false: quality switching passed.
- Fixed CORS/native-HTTP integration: use Capacitor's supported HTTP interceptor; preserve original XHR responseURL for relative HLS segment resolution (otherwise native interceptor address caused segment 404).
- Review fixes: completion latch prevents cleanup overwriting EOF; restart key makes Start over recreate playback; selected endpoint and explicit player preferences are respected; flush history before player destruction.
- Latest targeted ESLint, web build, native APK build/install: passed. Existing 13 mobile unit tests passed. These are not a full Artplayer codec/audio matrix.

Earlier iframe results below are historical and do not validate the replacement Artplayer. The later 20-minute signed-QA Artplayer soak is recorded above. The local test seeder has a bounded lifetime.

## Device and build

- Physical device: Infinix X6853, Android 16, API 36, arm64-v8a.
- Package: com.piratecinema.app; debug APK installed successfully with ADB.
- Rust 1.98.1, cargo-ndk 4.1.2, NDK r27c, JDK 21, SDK 36.
- Native librqbit 9.0.1 release `.so` built and included by Gradle.
- APK: android/app/build/outputs/apk/debug/app-debug.apk.

## Executed checks

- Rust host cargo check, clippy -D warnings, one initialize/idempotence/shutdown test: passed during engine implementation. These do not validate Android streaming.
- Android native release build and Gradle assembleDebug: passed, including the latest RuTube/menu changes.
- Mobile Vite build/Capacitor sync: passed.
- Targeted ESLint for RuTube components, helpers and mobile page: passed before the final stylesheet change.
- node --test tests/mobile-rutube.test.mjs tests/mobile-torrserver.test.mjs: 13 passed.
- Physical phone librqbit loopback root endpoint: HTTP 200 after fixing private DHT persistence path.
- Physical phone RuTube search “космос”: real named results and durations returned.
- Physical phone playlist https://rutube.ru/plst/1491780/: three real items loaded and saved locally.
- Embedded RuTube emitted real duration/currentTime events; first item duration 5001.32 seconds.
- A real seek command to 60 seconds resulted in persisted position 60.
- Manual Next changed iframe from first to second playlist video.
- Saved videos and queue remained after WebView reload and APK update.
- Compact navigation: all five buttons on one row at viewport width 360 CSS px, each 72×48 CSS px; nav height 58 CSS px; icons 19 px.

An early resume test showed initial player timing overwrote the saved position. The implementation now seeks on actual currentTime events and waits for acknowledgement before recording initial progress. A repeat phone test captured actual iframe currentTime 60.080502 seconds and matching persisted history after reopening. This checks resume at one position, not sustained playback.

Follow-up: navigation raised 20 CSS pixels; actual WebView height 812, nav bottom 792, all five buttons y=735. Fresh web/APK builds, APK install, 13 mobile tests and Rust clippy all-targets -D warnings passed.

## Outstanding

Outstanding: reconnect/network switching, long missing-piece waits, broader codec matrix and cancellation/cache fault injection. No claimed automatic RuTube playlist EOF test; only manual next was checked. RuTube requires network/provider availability. Saved playlist queues contain loaded pages; Load more must be used for longer playlists. RuTube history, playlists and queues are included in schema-2 JSON backups. The Android player now persists an auto-next preference for torrent series, but that new UI path still needs a repeat physical EOF check.

The broader pnpm test run previously had 11 failures: eight sandbox loopback EACCES failures, two unrelated desktop installer workflow assertions and one old mobile URL assertion (now updated and passing in the targeted suite). Full suite has not subsequently passed.

## Repeat commands (PowerShell, repository root)

```powershell
node --test tests/mobile-rutube.test.mjs tests/mobile-torrserver.test.mjs
pnpm run android:sync
cd android
.\gradlew.bat assembleDebug
cd ..
$adb = 'C:\Users\savay\AppData\Local\Android\Sdk\platform-tools\adb.exe'
& $adb devices -l
& $adb install -r android/app/build/outputs/apk/debug/app-debug.apk
& $adb shell am start -n com.piratecinema.app/.MainActivity
$appPid = & $adb shell pidof com.piratecinema.app
& $adb forward tcp:9223 "localabstract:webview_devtools_remote_$($appPid.Trim())"
node scripts/android-webview-eval.mjs 'document.body.innerText'
& $adb shell dumpsys meminfo com.piratecinema.app
& $adb logcat -d -s Librqbit:E AndroidRuntime:E
```

WebView evaluation uses the debug-only inspection endpoint. It must never be treated as a release interface. Do not clear app data or delete user torrents to repeat these tests.
