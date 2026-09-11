import test from "node:test";
import assert from "node:assert/strict";
import {readFileSync} from "node:fs";

test("both NSIS installers elevate before replacing locked application files",()=>{
  const config=readFileSync("electron-builder.yml","utf8");
  const hook=readFileSync("build/installer.nsh","utf8");
  assert.match(config,/perMachine:\s*true/);
  assert.match(config,/include:\s*build\/installer\.nsh/);
  assert.match(hook,/WM_CLOSE/);
  assert.match(hook,/taskkill\.exe.*Pirate Cinema\.exe/);
});

test("release workflow verifies bundled binaries and publishes updater metadata",()=>{
  const config=readFileSync("electron-builder.yml","utf8");const workflow=readFileSync(".github/workflows/release.yml","utf8");
  assert.match(config,/provider:\s*github/);
  assert.match(workflow,/TorrServer hash mismatch/);
  assert.match(workflow,/MPV hash mismatch/);
  assert.match(workflow,/release\/latest\.yml/);
});

test("Linux release verifies packages and creates freedesktop shortcuts",()=>{
  const config=readFileSync("electron-builder-linux.yml","utf8");
  const workflow=readFileSync(".github/workflows/release-linux.yml","utf8");
  const installer=readFileSync("scripts/install-linux.sh","utf8");
  assert.match(config,/target:\s*deb/);assert.match(config,/target:\s*rpm/);
  assert.match(config,/depends:\s*\[mpv, ffmpeg\]/);
  assert.match(workflow,/60c92e15ff2ac76d3dcebfe742ef2f0900905a6accef4329e7d3c66d8f0351dd/);
  assert.match(workflow,/SHA256SUMS/);assert.match(installer,/sha256sum -c/);
  assert.match(installer,/xdg-user-dir DESKTOP/);
});

test("Arch release publishes a checksummed PKGBUILD with pacman dependencies",()=>{
  const config=readFileSync("electron-builder-arch.yml","utf8");
  const workflow=readFileSync(".github/workflows/release-arch.yml","utf8");
  const pkgbuild=readFileSync("packaging/arch/PKGBUILD.in","utf8");
  assert.match(config,/target:\s*tar\.gz/);assert.match(workflow,/Special for Arch Linux/);
  assert.match(workflow,/tar -tzf/);assert.match(workflow,/s\/@SHA256@/);
  assert.match(pkgbuild,/depends=.*'mpv'.*'ffmpeg'/);assert.match(pkgbuild,/sha256sums=\('@SHA256@'\)/);
  assert.match(pkgbuild,/pirate-cinema\.desktop/);
});
