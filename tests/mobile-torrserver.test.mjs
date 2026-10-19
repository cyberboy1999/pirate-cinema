import test from "node:test";
import assert from "node:assert/strict";
import {serverCandidates} from "../app/mobile/torrserver.mjs";
import {androidRelease,writeStorageTransaction,safeMobileHistory,safeMobileMetadata} from "../app/mobile/torrserver.mjs";

test("Android discovery retains custom endpoints and both local server ports",()=>{
  const custom="http://192.168.1.42:9000";
  const hosts=serverCandidates(custom,"192.168.1.");
  assert.equal(hosts[0],custom);
  for(const host of ["127.0.0.1","10.0.2.2"])for(const port of [8090,8091])assert.ok(hosts.includes(`http://${host}:${port}`));
  assert.equal(hosts.length,new Set(hosts).size);
  assert.equal(serverCandidates("http://127.0.0.1:8091").length,4);
});
import {createMobileBackup,enableRutorSearch,episodeNumbers,magnetInfoHash,metadataQuery,mpvIntent,newerVersion,nextUnviewedFile,normalizeCinemeta,normalizeTorrent,parseMobileBackup,playbackHistoryEntry,probeTorrServer,removeTorrentHistory,sortEpisodeFiles,streamUrl,updatePlaybackHistory} from "../app/mobile/torrserver.mjs";

test("mobile client reads torrent files and builds librqbit player links",()=>{
  const item=normalizeTorrent({hash:"a".repeat(40),title:"Series",data:JSON.stringify({TorrServer:{Files:[{id:7,path:"S01/E02.mkv",length:42},{id:8,path:"cover.jpg",length:2}]}})});
  assert.equal(item.files.length,1);assert.equal(item.files[0].id,7);
  const url=streamUrl("http://127.0.0.1:8090/",item.hash,item.files[0]);
  assert.match(url,/\/torrents\/[a-f0-9]{40}\/stream\/7\/E02\.mkv$/);assert.match(mpvIntent(url,item.title),/package=is\.xyz\.mpv/);
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

test("mobile client updates and removes local per-file history",()=>{
  const history={"a:1":{hash:"a",positionMs:50,completed:false},"b:2":{hash:"b",positionMs:20,completed:false}};
  assert.deepEqual(updatePlaybackHistory(history,"a:1",{completed:true,positionMs:0})["a:1"],{hash:"a",positionMs:0,completed:true});
  assert.deepEqual(removeTorrentHistory(history,"a"),{"b:2":history["b:2"]});
});

test("mobile client prepares release names and Cinemeta metadata",()=>{
  assert.equal(metadataQuery("Interstellar.2014.UHD.BDRip.2160p.4K"),"Interstellar 2014");
  assert.equal(metadataQuery("Shogun.S01E02.1080p.WEB-DL"),"Shogun");
  assert.deepEqual(normalizeCinemeta({id:"tt1",name:"Movie",releaseInfo:"2024",poster:"p",imdbRating:"7.5"}),{id:"tt1",title:"Movie",year:2024,poster:"p",backdrop:"",description:"",rating:7.5,genres:[]});
});

test("mobile client validates portable local backups",()=>{
  const hash="a".repeat(40),entry={hash,fileId:0,fileName:"movie.mkv",title:"Movie",positionMs:42,durationMs:60,completed:false,lastPlayedAt:"2026-09-17T00:00:00.000Z"};
  const content=createMobileBackup({serverUrl:"http://127.0.0.1:8090",player:"mpv",history:{[`${hash}:0`]:entry},metadata:{}},"2026-09-17T00:00:00.000Z");
  assert.equal(parseMobileBackup(content).history[`${hash}:0`].hash,hash);
  assert.throws(()=>parseMobileBackup('{"schema":2}'),/Неподдерживаемая/);
});

test("mobile client groups episodes and finds the next unviewed file",()=>{
  assert.deepEqual(episodeNumbers("Season 2/Show.S02E03.mkv"),{season:2,episode:3});
  const files=sortEpisodeFiles([{id:8,path:"S02E01.mkv",season:2,episode:1},{id:7,path:"S01E02.mkv",season:1,episode:2}]);
  assert.deepEqual(files.map(file=>file.id),[7,8]);
  assert.equal(nextUnviewedFile(files,{"h:7":{completed:true}},"h",7).id,8);
});

test("mobile client recognizes duplicate hex and base32 magnet hashes",()=>{
  assert.equal(magnetInfoHash(`magnet:?xt=urn:btih:${"A".repeat(40)}`),"a".repeat(40));
  assert.equal(magnetInfoHash("magnet:?xt=urn:btih:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"),"0".repeat(40));
  assert.equal(magnetInfoHash("https://example.test/not-a-magnet"),null);
  assert.equal(magnetInfoHash(`https://evil.test/?xt=urn:btih:${"a".repeat(40)}`),null);
});

test("mobile update comparison ignores older GitHub releases",()=>{
  assert.equal(newerVersion("v0.5.9","0.5.8"),true);
  assert.equal(newerVersion("v0.5.7","0.5.8"),false);
  assert.equal(newerVersion("v0.5.8","0.5.8"),false);
  assert.equal(newerVersion("android-v0.6.0-rc.2","0.6.0-rc.1"),true);
  assert.equal(newerVersion("android-v0.6.0","0.6.0-rc.2"),true);
  assert.equal(newerVersion("android-v0.6.0-rc.2","0.6.0"),false);
  const release={tag_name:"android-v0.6.0-rc.2",prerelease:true,assets:[{name:"Pirate-Cinema-Android-0.6.0-rc.2-arm64-v8a.apk",browser_download_url:"https://github.com/cyberboy1999/pirate-cinema/releases/download/android-v0.6.0-rc.2/a.apk"}]};
  assert.ok(androidRelease([release],"0.6.0-rc.1"));assert.equal(androidRelease([release],"0.6.0"),null);
  assert.equal(androidRelease([{...release,assets:[{...release.assets[0],browser_download_url:"https://evil.example/a.apk"}]}],"0.6.0-rc.1"),null);
});

test("mobile backup includes RuTube and validates records before restore",()=>{
  const video={id:"a".repeat(32),url:`https://rutube.ru/video/${"a".repeat(32)}/`,title:"Owned fixture",position:42,duration:60,completed:false};
  const backup=createMobileBackup({serverUrl:"http://127.0.0.1:3030",player:"mpv",history:{},metadata:{},rutube:[video],rutubeQueue:[video],rutubePlaylists:[{url:"https://rutube.ru/plst/123/",videos:[video]}]});
  const restored=parseMobileBackup(backup);assert.equal(restored.schema,2);assert.equal(restored.rutube[0].position,42);assert.equal(restored.rutubePlaylists[0].videos[0].id,video.id);
  assert.throws(()=>parseMobileBackup(JSON.stringify({...restored,history:[]})));
  assert.throws(()=>parseMobileBackup(JSON.stringify({...restored,history:{bad:{positionMs:-1}}})));
  assert.throws(()=>parseMobileBackup(JSON.stringify({...restored,history:{a:{},b:{}}})));
  assert.throws(()=>parseMobileBackup(JSON.stringify({...restored,metadata:{["a".repeat(40)]:{genres:"bad"}}})));
  assert.throws(()=>parseMobileBackup(JSON.stringify({...restored,serverUrl:"javascript:bad"})));
  assert.throws(()=>parseMobileBackup(" ".repeat(5_000_001)));
  assert.deepEqual(parseMobileBackup(JSON.stringify({...restored,schema:1,rutube:undefined})).rutube,[]);
});

test("quota failure rolls back backup writes and malformed startup data cannot crash render",()=>{
  const values=new Map([["history","original"],["metadata","old"]]);
  const storage={getItem:key=>values.get(key)??null,removeItem:key=>values.delete(key),setItem(key,value){if([...values.entries()].filter(([name])=>name!==key).reduce((size,[,text])=>size+text.length,0)+value.length>30)throw new Error("QuotaExceeded");values.set(key,value)}};
  assert.throws(()=>writeStorageTransaction(storage,{safety:"snapshot",history:"replacement",metadata:"x".repeat(40)}),/Quota/);
  assert.deepEqual(Object.fromEntries(values),{history:"original",metadata:"old"});
  writeStorageTransaction(storage,{history:"new",metadata:"updated"});assert.equal(values.get("history"),"new");
  assert.deepEqual(safeMobileHistory({a:{},b:{}}),{});assert.deepEqual(safeMobileMetadata({a:{genres:"bad"}}),{});
});

test("mobile TorrServer discovery probes the LAN in bounded batches",async()=>{
  const hosts=Array.from({length:40},(_,index)=>`http://192.168.1.${index+1}:8090`);
  let active=0,peak=0;
  const found=await probeTorrServer(hosts,async url=>{active++;peak=Math.max(peak,active);await new Promise(resolve=>setTimeout(resolve,1));active--;return {ok:url===`${hosts[20]}/echo`}});
  assert.equal(found,hosts[20]);assert.ok(peak<=16);
});
