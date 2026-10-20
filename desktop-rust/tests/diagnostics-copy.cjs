const fs = require('node:fs');
const assert = require('node:assert/strict');
const path = require('node:path');
const source = fs.readFileSync(path.join(__dirname, '../src/bin/dioxus.rs'), 'utf8');
const start = source.indexOf('fn system_diagnostics()');
const end = source.indexOf('\n#[cfg(windows)]', start);
assert.ok(start >= 0 && end > start, 'diagnostics function exists');
const diagnostics = source.slice(start, end);

assert.match(diagnostics, /desktop_engine::health_version/);
assert.match(diagnostics, /web_stream::executable/);
assert.match(diagnostics, /integrity_check/);
assert.match(diagnostics, /read_library/);
assert.doesNotMatch(diagnostics, /TorrServer|MPV/);
console.log('Diagnostics: librqbit, FFmpeg, SQLite and local catalogue: OK');
