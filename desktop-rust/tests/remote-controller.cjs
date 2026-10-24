const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

const source = fs.readFileSync(path.join(__dirname, '../src/bin/dioxus.rs'), 'utf8');
const template = source.split('let script = format!(')[1].split('r#"')[1].split('"#')[0];

async function transition(initial) {
  let script = template
    .replaceAll('{commands}', JSON.stringify([{ action: 'fullscreen' }]))
    .replaceAll('{title}', JSON.stringify('Video'))
    .replaceAll('{{', '{')
    .replaceAll('}}', '}');
  const art = {
    fullscreen: false,
    fullscreenWeb: initial,
    playing: false,
    currentTime: 0,
    duration: 60,
    volume: 1,
  };
  const audio = { options: [], value: '' };
  const document = {
    fullscreenElement: initial ? {} : null,
    querySelector: selector => selector === '.pc-audio' ? audio : null,
    exitFullscreen: async () => { document.fullscreenElement = null; },
  };
  let status;
  const run = new (Object.getPrototypeOf(async function () {}).constructor)(
    'window', 'document', 'dioxus', script,
  );
  await run({ __pirateCinemaArt: art }, document, { send: value => { status = value; } });
  return { art, status };
}

(async () => {
  const exited = await transition(true);
  assert.equal(exited.art.fullscreen, false);
  assert.equal(exited.art.fullscreenWeb, false);
  assert.equal(exited.status.fullscreen, false);

  const entered = await transition(false);
  assert.equal(entered.art.fullscreen, false);
  assert.equal(entered.art.fullscreenWeb, true);
  assert.equal(entered.status.fullscreen, true);
  console.log('Remote fullscreen: local web-fullscreen exits in one press and re-enters cleanly: OK');
})().catch(error => { console.error(error); process.exitCode = 1; });
