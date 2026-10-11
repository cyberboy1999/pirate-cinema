const fs = require('node:fs');
const vm = require('node:vm');
const assert = require('node:assert/strict');
const source = fs.readFileSync(require('node:path').join(__dirname, '../src/bin/dioxus.rs'), 'utf8');
const position = source.indexOf("const text={text}+'\\nWebView UA: '");
assert.ok(position >= 0, 'actual diagnostics copy handler exists');
const start = source.lastIndexOf('r#"', position) + 3;
const script = source.slice(start, source.indexOf('"#', position))
  .replace('{text}', JSON.stringify('public report'))
  .replaceAll('{{', '{').replaceAll('}}', '}');
(async () => {
  for (const mode of ['native', 'fallback', 'denied']) {
    let copied = '', result, removed = false;
    const element = { value: '', style: {}, select() {}, remove() { removed = true; } };
    await vm.runInNewContext(script, {
      navigator: { userAgent: 'WebView test', clipboard: { async writeText(text) {
        if (mode !== 'native') throw new Error('permission');
        copied = text;
      } } },
      document: { createElement() { return element; }, body: { append() {} }, execCommand(command) {
        assert.equal(command, 'copy');
        copied = element.value;
        return mode === 'fallback';
      } },
      dioxus: { send(value) { result = value; } },
    });
    assert.equal(result, mode !== 'denied');
    assert.equal(copied, 'public report\nWebView UA: WebView test');
    assert.equal(removed, mode !== 'native');
  }
  console.log('Diagnostics copy: native, fallback and permission failure: OK');
})().catch(error => { console.error(error); process.exitCode = 1; });
