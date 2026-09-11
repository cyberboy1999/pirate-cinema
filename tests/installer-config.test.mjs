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
