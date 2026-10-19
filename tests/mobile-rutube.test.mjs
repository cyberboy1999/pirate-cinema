import test from "node:test";
import assert from "node:assert/strict";
import {rutubeVideo,rutubeProgress,savedRutubeVideos} from "../app/mobile/rutube.mjs";
const id="9e4cd81a6b2566e9d949881dbb53905e",url=`https://rutube.ru/video/${id}/`;
test("RuTube canonicalizes video, shorts and embed links and rejects other origins",()=>{
  for(const path of ["video","shorts","play/embed"])assert.deepEqual(rutubeVideo(`https://rutube.ru/${path}/${id}/?t=2`),{id,url});
  for(const bad of [`http://rutube.ru/video/${id}`,`https://rutube.ru.evil.test/video/${id}`,`https://evil.test/video/${id}`,`https://user@rutube.ru/video/${id}`,"https://rutube.ru/video/invalid","javascript:alert(1)"])assert.throws(()=>rutubeVideo(bad));
});
test("RuTube library survives serialization, deduplicates IDs and validates restored history",()=>{
  const item={url,title:"Test",position:42,duration:120,completed:false};
  assert.deepEqual(savedRutubeVideos(JSON.parse(JSON.stringify([item,item,{url:"https://evil.test"}]))),[{...item,id}]);
  assert.equal(savedRutubeVideos([{...item,position:-1}])[0].position,0);
});
test("RuTube records real player progress and completion without accepting malformed events",()=>{
  const item={url,position:0,duration:0,completed:false};
  const progress=rutubeProgress(item,JSON.stringify({type:"player:currentTime",data:{time:42}}));assert.equal(progress.position,42);
  assert.equal(rutubeProgress(progress,{type:"player:durationChange",data:{duration:120}}).duration,120);
  assert.deepEqual(rutubeProgress(progress,{type:"player:playComplete"}),{...item,completed:true});
  for(const message of ["invalid",{type:"player:currentTime",data:{time:-1}},{type:"player:currentTime",data:{time:Infinity}}])assert.equal(rutubeProgress(progress,message),progress);
});
