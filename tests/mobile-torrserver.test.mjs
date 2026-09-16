import test from "node:test";
import assert from "node:assert/strict";
import {mpvIntent,normalizeTorrent,streamUrl} from "../app/mobile/torrserver.mjs";

test("mobile client reads TorrServer files and builds external player links",()=>{
  const item=normalizeTorrent({hash:"a".repeat(40),title:"Series",data:JSON.stringify({TorrServer:{Files:[{id:7,path:"S01/E02.mkv",length:42},{id:8,path:"cover.jpg",length:2}]}})});
  assert.equal(item.files.length,1);assert.equal(item.files[0].id,7);
  const url=streamUrl("http://127.0.0.1:8090/",item.hash,item.files[0]);
  assert.match(url,/index=7&play$/);assert.match(mpvIntent(url,item.title),/package=is\.xyz\.mpv/);
});
