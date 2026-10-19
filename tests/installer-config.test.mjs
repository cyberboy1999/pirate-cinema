import test from "node:test";
import assert from "node:assert/strict";
import {readFileSync} from "node:fs";

const read = (path) => readFileSync(path, "utf8");

test("Windows installer is elevated and carries the complete Rust runtime", () => {
  const installer = read("desktop-rust/installer.nsi");
  assert.match(installer, /RequestExecutionLevel admin/);
  assert.match(installer, /File "\$\{APP_SOURCE\}\\pirate-cinema\.exe"/);
  assert.match(installer, /File "\$\{APP_SOURCE\}\\vc_redist\.x64\.exe"/);
  assert.match(installer, /File "\$\{APP_SOURCE\}\\ffmpeg\\ffmpeg\.exe"/);
  assert.doesNotMatch(installer, /File .*\\(?:torrserver|mpv)\\/i);
  assert.match(installer, /CreateShortcut "\$DESKTOP\\Pirate Cinema\.lnk"/);
});

test("release workflow verifies runtimes and publishes only the supported artifacts", () => {
  const workflow = read(".github/workflows/release.yml");
  assert.match(workflow, /FFmpeg hash mismatch/);
  assert.match(workflow, /CC0FF0EB1DC3F5188AE6300FAEF32BF5BEEBA4BDD6E8E445A9184072096B713B/);
  assert.match(workflow, /VC\+\+ Redistributable hash mismatch/);
  assert.match(workflow, /pnpm install --frozen-lockfile/);
  assert.match(workflow, /pnpm test/);
  assert.match(workflow, /pnpm run lint/);
  assert.match(workflow, /appimagetool-x86_64\.AppImage/);
  assert.match(workflow, /sha256sum -c/);
  assert.match(workflow, /desktop-rust\/packaging\/build-appimage\.sh/);
  assert.match(workflow, /needs: \[windows, linux, arch-smoke\]/);
  assert.match(workflow, /xvfb-run/);
  assert.doesNotMatch(workflow, /nsis-web|PKGBUILD|--linux deb rpm/);
});

test("release inputs never include a user database or TorrServer state", () => {
  const installer = read("desktop-rust/installer.nsi");
  const workflow = read(".github/workflows/release.yml");
  for (const source of [installer, workflow]) {
    assert.doesNotMatch(source, /config\.db/);
    assert.doesNotMatch(source, /viewed\.json/);
  }
  assert.doesNotMatch(workflow, /bootstrap\/resources\/torrserver/);
  assert.doesNotMatch(workflow, /TorrServer-(?:gst-)?(?:windows|linux)/);
});

test("AppImage build keeps the required runtime layout and launcher", () => {
  const script = read("desktop-rust/packaging/build-appimage.sh");
  assert.match(script, /mkdir -p "\$appdir\/usr\/bin"/);
  assert.match(script, /install -m755 "\$binary" "\$appdir\/usr\/bin\/pirate-cinema"/);
  assert.doesNotMatch(script, /TorrServer|mpv/i);
  assert.match(script, /libxdo\.so\.3/);
  assert.match(script, /LD_LIBRARY_PATH/);
  assert.match(script, /Pirate-Cinema-\$version-x86_64\.AppImage/);
});
