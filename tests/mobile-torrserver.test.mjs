import test from "node:test";
import assert from "node:assert/strict";
import {enableRutorSearch,mpvIntent,normalizeTorrent,playbackHistoryEntry,streamUrl} from "../app/mobile/torrserver.mjs";

test("mobile client reads TorrServer files and builds external player links",()=>{
  const item=normalizeTorrent({hash:"a".repeat(40),title:"Series",data:JSON.stringify({TorrServer:{Files:[{id:7,path:"S01/E02.mkv",length:42},{id:8,path:"cover.jpg",length:2}]}})});
  assert.equal(item.files.length,1);assert.equal(item.files[0].id,7);
  const url=streamUrl("http://127.0.0.1:8090/",item.hash,item.files[0]);
  assert.match(url,/index=7&play$/);assert.match(mpvIntent(url,item.title),/package=is\.xyz\.mpv/);
});

test("mobile client enables RuTor without replacing other TorrServer settings",()=>{
  assert.deepEqual(enableRutorSearch({CacheSize:64,EnableRutorSearch:false}),{CacheSize:64,EnableRutorSearch:true});
  assert.equal(enableRutorSearch({EnableRutorSearch:true}),null);
});

test("mobile client stores mpv result and clears position after completion",()=>{
  const media={hash:"a".repeat(40),fileId:7,fileName:"Episode.mkv",title:"Series"};
  const saved=playbackHistoryEntry(null,{ok:true,positionMs:125000,durationMs:1500000},media,"2026-09-17T00:00:00.000Z");
  assert.equal(saved.positionMs,125000);assert.equal(saved.durationMs,1500000);assert.equal(saved.completed,false);
  assert.equal(playbackHistoryEntry(saved,{ok:true,completed:true},media).positionMs,0);
});
