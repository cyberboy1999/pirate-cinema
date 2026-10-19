import assert from "node:assert/strict";
import {readFile,open} from "node:fs/promises";
const [source,torrent]=process.argv.slice(2);
if(!source||!torrent)throw new Error("usage: node scripts/test-android-torrent-stream.mjs <fixture.mp4> <fixture.torrent>; adb forward tcp:3030 tcp:3030; adb reverse tcp:4242 tcp:4242");
const base="http://127.0.0.1:3030",start=performance.now();
const added=await fetch(`${base}/torrents?overwrite=true&initial_peers=127.0.0.1%3A4242`,{method:"POST",body:await readFile(torrent),signal:AbortSignal.timeout(60000)});
assert.equal(added.status,200,await added.clone().text());
const result=await added.json(),hash=result.details.info_hash,url=`${base}/torrents/${hash}/stream/0`;
console.log(JSON.stringify({metadataMs:Math.round(performance.now()-start),hash,url}));
const file=await open(source,"r");
try {
  const size=(await file.stat()).size;
  const head=await fetch(url,{method:"HEAD",signal:AbortSignal.timeout(15000)});
  assert.equal(head.status,200);assert.equal(Number(head.headers.get("content-length")),size);assert.equal(head.headers.get("accept-ranges"),"bytes");assert.equal((await head.arrayBuffer()).byteLength,0);
  console.log("PASS HEAD 200 length and Accept-Ranges");
  for(const offset of [0,Math.floor(size/2),size-4096,131072,65536]){
    const response=await fetch(url,{headers:{range:`bytes=${offset}-${offset+4095}`},signal:AbortSignal.timeout(60000)});
    assert.equal(response.status,206);assert.equal(response.headers.get("content-range"),`bytes ${offset}-${offset+4095}/${size}`);
    const actual=Buffer.from(await response.arrayBuffer()),expected=Buffer.alloc(4096);await file.read(expected,0,4096,offset);assert.deepEqual(actual,expected);
    console.log(`PASS Range 206 exact bytes ${offset}`);
  }
  const invalid=await fetch(url,{headers:{range:`bytes=${size}-`},signal:AbortSignal.timeout(10000)});assert.equal(invalid.status,416);console.log("PASS invalid Range 416");
  await Promise.all([0,65536,131072].map(async offset=>{const r=await fetch(url,{headers:{range:`bytes=${offset}-${offset+1023}`},signal:AbortSignal.timeout(30000)});assert.equal(r.status,206);assert.equal((await r.arrayBuffer()).byteLength,1024)}));console.log("PASS concurrent requests");
  const abort=new AbortController(),whole=await fetch(url,{signal:abort.signal});assert.equal(whole.status,200);const reader=whole.body.getReader();assert.ok((await reader.read()).value.length>0);abort.abort();console.log("PASS GET 200 and client cancellation");
  const stats=await(await fetch(`${base}/torrents/${hash}/stats/v1`)).json();console.log(JSON.stringify({stats,totalMs:Math.round(performance.now()-start)}));
}finally{await file.close()}
