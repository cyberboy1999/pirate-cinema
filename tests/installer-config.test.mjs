import test from "node:test";
import assert from "node:assert/strict";
import {readFileSync} from "node:fs";

const read = (path) => readFileSync(path, "utf8");

test("Windows installer is elevated and carries the complete Rust runtime", () => {
  const installer = read("desktop-rust/installer.nsi");
  assert.match(installer, /RequestExecutionLevel admin/);
  assert.match(installer, /File "\$\{APP_SOURCE\}\\pirate-cinema\.exe"/);
  assert.match(installer, /File "\$\{APP_SOURCE\}\\vc_redist\.x64\.exe"/);
  assert.match(installer, /File "\$\{APP_SOURCE\}\\torrserver\\TorrServer-windows-amd64\.exe"/);
  assert.match(installer, /CreateShortcut "\$DESKTOP\\Pirate Cinema\.lnk"/);
});

test("release workflow verifies runtimes and publishes only the supported artifacts", () => {
  const workflow = read(".github/workflows/release.yml");
  assert.match(workflow, /TorrServer hash mismatch/);
  assert.match(workflow, /MPV hash mismatch/);
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
});

test("AppImage build keeps the required runtime layout and launcher", () => {
  const script = read("desktop-rust/packaging/build-appimage.sh");
  assert.match(script, /install -m755 "\$binary" "\$appdir\/usr\/bin\/pirate-cinema"/);
  assert.match(script, /TorrServer-linux-amd64/);
  assert.match(script, /libxdo\.so\.3/);
  assert.match(script, /LD_LIBRARY_PATH/);
  assert.match(script, /Pirate-Cinema-\$version-x86_64\.AppImage/);
});
