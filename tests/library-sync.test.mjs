import test from "node:test";
import assert from "node:assert/strict";
import {MediaDatabase} from "../server/database.mjs";
import {createLibrarySync} from "../server/library-sync.mjs";

test("full sync refreshes every description, keeps file history and cached data on provider failures",async()=>{
  const db=new MediaDatabase(":memory:"),hash="a".repeat(40),second="b".repeat(40);
  const state={};let calls=0,fail=false;
  const client={echo:async()=>"MatriX",listTorrents:async()=>[{hash,title:"Movie.2024"},{hash:second,title:"Other.2024"}],
    listViewed:async()=>[{hash,timecode:0,duration:1000}]};
  const metadata={findBestMatch:async(title,{refresh})=>{
    calls++;if(fail)throw new Error("provider unavailable");
    return {title,provider:"cinemeta",providerId:title,confidence:1,overview:refresh?"Updated description":"First description",genres:["Drama"],posterUrl:"https://example.invalid/poster.jpg"};
  }};
  const sync=createLibrarySync({db,client,metadata,cachePoster:async()=>"cached.jpg",state});
  try{
    await sync.synchronize();
    assert.equal(calls,2);assert.equal(sync.item(hash).overview,"First description");
    db.markFilePlayed(hash,{fileIndex:42,fileName:"movie.mkv"});
    db.updateFileProgress(hash,42,{timecode:123,duration:1000});
    db.setFileViewed(hash,42,false);
    await sync.synchronize();
    assert.equal(calls,2);
    await sync.synchronize(true);
    assert.equal(calls,4);assert.equal(sync.item(hash).overview,"Updated description");
    assert.equal(sync.item(hash).progress,12);assert.equal(sync.item(hash).isStarted,true);
    assert.equal(db.listFileHistory(hash)[0].isWatched,0);
    assert.deepEqual(state.syncProgress,{processed:2,total:2,failed:0,full:true});
    fail=true;await sync.synchronize(true);
    assert.equal(state.online,true);assert.equal(state.syncProgress.failed,2);
    assert.equal(sync.item(hash).overview,"Updated description");
    assert.equal(sync.item(hash).posterPath,"/api/posters/cached.jpg");
    assert.deepEqual(sync.item(hash).genres,["Drama"]);
    client.listTorrents=async()=>{throw new Error("bad response")};
    await sync.synchronize(true);
    assert.equal(db.list().length,2);
    assert.equal(db.listFileHistory(hash)[0].playbackTimecode,123);
  }finally{db.close()}
});

test("manual full sync waits for background sync and then refreshes all metadata",async()=>{
  const db=new MediaDatabase(":memory:"),hash="c".repeat(40),modes=[];
  let release;const gate=new Promise(r=>{release=r});
  const sync=createLibrarySync({db,state:{},cachePoster:async()=>null,
    client:{echo:async()=>"MatriX",listTorrents:async()=>[{hash,title:"Movie"}],listViewed:async()=>[]},
    metadata:{findBestMatch:async(_title,{refresh})=>{modes.push(refresh);if(!refresh)await gate;return {confidence:1,provider:"cinemeta",providerId:"1",title:"Movie",overview:"Description"}}}});
  try{
    const background=sync.synchronize(false);
    const manual=sync.synchronize(true);
    release();await Promise.all([background,manual]);
    assert.deepEqual(modes,[false,true]);
    assert.equal(sync.item(hash).overview,"Description");
  }finally{db.close()}
});

test("manual metadata query survives sync and drives enrichment",async()=>{
  const db=new MediaDatabase(":memory:"),hash="d".repeat(40),queries=[];
  const sync=createLibrarySync({db,state:{},cachePoster:async()=>"manual.jpg",
    client:{echo:async()=>"MatriX",listTorrents:async()=>[{hash,title:"Bad.Release.Name.2160p"}],listViewed:async()=>[]},
    metadata:{findBestMatch:async title=>{queries.push(title);return {confidence:1,provider:"cinemeta",providerId:"tt1",title:"Dune",year:2021,posterUrl:"https://example.invalid/dune.jpg",overview:"Description"}}}});
  try{
    await sync.synchronize();db.setMetadataQuery(hash,"Dune 2021");await sync.enrich(hash,true);await sync.synchronize(true);
    assert.deepEqual(queries,["Bad.Release.Name.2160p","Dune 2021","Dune 2021"]);
    assert.equal(sync.item(hash).title,"Dune 2021");assert.equal(sync.item(hash).overview,"Description");assert.equal(sync.item(hash).posterPath,"/api/posters/manual.jpg");
  }finally{db.close()}
});
