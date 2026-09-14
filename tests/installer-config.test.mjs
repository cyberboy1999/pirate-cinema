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
  const workflow=readFileSync(".github/workflows/release.yml","utf8");
  const installer=readFileSync("scripts/install-linux.sh","utf8");
  assert.match(config,/target:\s*deb/);assert.match(config,/target:\s*rpm/);
  assert.match(config,/depends:\s*\[mpv, ffmpeg\]/);
  assert.match(workflow,/8b61aa8e85eb6c5caee3b484160f27d82da28bc6b2f0d6703a8914293824afe0/);
  assert.match(workflow,/SHA256SUMS/);assert.match(installer,/sha256sum -c/);
  assert.match(installer,/xdg-user-dir DESKTOP/);
});

test("Arch release publishes a checksummed PKGBUILD with pacman dependencies",()=>{
  const config=readFileSync("electron-builder-arch.yml","utf8");
  const workflow=readFileSync(".github/workflows/release.yml","utf8");
  const pkgbuild=readFileSync("packaging/arch/PKGBUILD.in","utf8");
  assert.match(config,/target:\s*tar\.gz/);assert.match(workflow,/Build unified release/);
  assert.match(workflow,/tar -xzf/);assert.match(workflow,/s\/@SHA256@/);
  assert.match(pkgbuild,/depends=.*'mpv'.*'ffmpeg'/);assert.match(pkgbuild,/sha256sums=\('@SHA256@'\)/);
  assert.match(pkgbuild,/pirate-cinema\.desktop/);
  assert.match(pkgbuild,/chmod 755 .*TorrServer-linux-amd64/);
  assert.doesNotMatch(readFileSync("electron/main.mjs","utf8"),/chmodSync/);
});

test("unified release starts with empty user databases",()=>{
  const windows=readFileSync("electron-builder.yml","utf8");
  const linux=readFileSync("electron-builder-linux.yml","utf8");
  const workflow=readFileSync(".github/workflows/release.yml","utf8");
  for(const source of [windows,linux]){assert.doesNotMatch(source,/config\.db/);assert.doesNotMatch(source,/viewed\.json/)}
  assert.doesNotMatch(workflow,/bootstrap\/resources\/torrserver\/\*/);
  assert.match(workflow,/needs: \[windows, linux\]/);
});

test("Electron keeps the app available from the system tray",()=>{
  const main=readFileSync("electron/main.mjs","utf8");
  assert.match(main,/new Tray\(/);
  assert.match(main,/mainWindow\.hide\(\)/);
  assert.match(main,/label:"Выйти"/);
});
