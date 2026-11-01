// Exercise the embedded audio-selection position calculation.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const source = fs.readFileSync(require('node:path').join(__dirname, '../src/bin/dioxus.rs'), 'utf8');
const block = source.match(/const savedStart=Number\(next\.searchParams\.get\('seconds'\)\)[\s\S]*?next\.searchParams\.set\('seconds', String\(Math\.floor\(audioPosition\)\)\);/);
assert.ok(block, 'embedded audio resume calculation exists');
const selectPosition = new Function('next', 'art', 'restoringSavedAudio', 'sourceReady', block[0]);
for (const [restoring, ready, current, expected] of [[true,false,0,87],[false,true,142,142],[true,true,142,142]]) {
  const url = new URL('http://127.0.0.1/playlist.m3u8?seconds=87');
  selectPosition(url, {currentTime:current}, restoring, ready);
  assert.equal(Number(url.searchParams.get('seconds')), expected);
}
console.log('Saved audio restoration preserves pending resume; manual audio uses current time: OK');