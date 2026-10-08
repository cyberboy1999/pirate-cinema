// Run with node tests/player-switch.cjs. Exercise the actual embedded reuse path.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const source = fs.readFileSync(path.join(__dirname, '../src/bin/dioxus.rs'), 'utf8');
let script = source.split('let script = format!(')[1].split('r#"')[1].split('"#,')[0];
for (const [key, value] of Object.entries({
  selector: '"torrent-2"', episodes: '[]', torrent_hash: '"torrent"',
  current_file_id: '2', next_episode_id: '3', hls_js: '', artplayer_js: '',
  player_state_js: '', audio_preference: 'null', auto_next: 'true',
})) script = script.replaceAll(`{${key}}`, value);
script = script.replaceAll('{{', '{').replaceAll('}}', '}');
const AsyncFunction = Object.getPrototypeOf(async function () {}).constructor;
const container = {};
const player = { fullscreen: true, destroy() { assert.fail('player was destroyed'); } };
let calls = 0;
const window = {
  __pirateCinemaContainer: container, __pirateCinemaTorrent: 'torrent',
  __pirateCinemaArt: player,
  __pirateCinemaSwitch: async (id, next) => { assert.equal(id, 2); assert.equal(next, 3); calls++; },
};
new AsyncFunction('window', 'document', script)(window, { querySelector: () => container })
  .then(() => {
    assert.equal(calls, 1);
    assert.equal(window.__pirateCinemaArt, player);
    assert.equal(player.fullscreen, true);
    console.log('Same-container episode switch retains player and fullscreen: OK');
  }).catch(error => { console.error(error); process.exitCode = 1; });
