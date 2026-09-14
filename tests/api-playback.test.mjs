import test from "node:test";
import assert from "node:assert/strict";
import {createServer} from "node:http";
import {spawn} from "node:child_process";
import {once} from "node:events";
import {mkdtempSync,rmSync} from "node:fs";
import {tmpdir} from "node:os";
import {join,resolve} from "node:path";
import {MediaDatabase} from "../server/database.mjs";

test("local API exposes per-file progress, persists manual viewed changes and rejects invalid playback",async()=>{
  const folder=mkdtempSync(join(tmpdir(),"pirate-cinema-api-test-"));
  const hash="d".repeat(40);
  const db=new MediaDatabase(join(folder,"media.db"));
  db.upsert({torrentHash:hash,torrentName:"Test",title:"Test"});
  db.markFilePlayed(hash,{fileIndex:42,fileName:"S01E02.mkv"});
  db.updateFileProgress(hash,42,{timecode:123,duration:1000});
  db.close();
  const torrents=[{hash,title:"Test",file_stats:[{id:9,path:"Test/S01E10.mkv",length:100},{id:42,path:"Test/S01E02.mkv",length:100}]}];
  let addedCount=0;
  const torr=createServer(async(req,res)=>{
    const chunks=[];for await(const chunk of req)chunks.push(chunk);
    const body=chunks.length?JSON.parse(Buffer.concat(chunks).toString()):{};
    if(req.url==="/torrents"&&body.action==="add"){
      addedCount++;
      const addedHash="e".repeat(40);
      torrents.push({hash:addedHash,title:body.title});
      res.setHeader("content-type","application/json");return res.end(JSON.stringify({hash:addedHash}));
    }
    res.setHeader("content-type","application/json");
    res.end(JSON.stringify(req.url==="/echo"?"TestServer":req.url==="/torrents"?torrents:[]));
  });
  await new Promise(r=>torr.listen(0,"127.0.0.1",r));
  const child=spawn(process.execPath,["server/api.mjs"],{cwd:resolve("."),windowsHide:true,
    env:{...process.env,TORR_LOCAL_API_PORT:"0",TORR_DATA_DIR:folder,TORRSERVER_URL:"http://127.0.0.1:"+torr.address().port,MPV_PATH:process.execPath,REMOTE_POSTERS:"0",TMDB_API_KEY:"",TMDB_READ_TOKEN:""},
    stdio:["ignore","pipe","pipe"],
  });
  const exited=once(child,"exit");let logs="";
  child.stdout.setEncoding("utf8");child.stdout.on("data",chunk=>{logs+=chunk});
  child.stderr.on("data",chunk=>{logs+=String(chunk)});
  try{
    const deadline=Date.now()+10000;
    while(!/local-api.*127.0.0.1:(\d+)/.test(logs)){
      if(child.exitCode!==null||Date.now()>deadline)throw new Error(logs||"API did not start");
      await new Promise(r=>setTimeout(r,25));
    }
    const base="http://127.0.0.1:"+logs.match(/local-api.*127.0.0.1:(\d+)/)[1];
    async function post(path,body){return fetch(base+path,{method:"POST",headers:{"content-type":"application/json"},body:JSON.stringify(body)})}
    const sync=await post("/api/sync",{});assert.equal((await sync.json()).state.online,true);
    const initialSettings=await (await fetch(base+"/api/settings")).json();assert.equal(initialSettings.onboardingComplete,false);assert.equal(initialSettings.playerType,"mpv");assert.equal(initialSettings.language,"ru");
    assert.equal((await post("/api/settings",{playerType:"external",playerPath:"C:\\missing-player.exe"})).status,400);
    const savedSettings=await (await post("/api/settings",{onboardingComplete:true,language:"en",playerType:"external",playerPath:process.execPath})).json();assert.equal(savedSettings.onboardingComplete,true);assert.equal(savedSettings.language,"en");assert.equal(savedSettings.playerPath,process.execPath);
    assert.equal((await post("/api/settings",{playerType:"mpv"})).status,200);
    const route="/api/torrents/"+hash+"/files";
    const diagnostics=await (await fetch(base+"/api/diagnostics")).json();assert.equal(diagnostics.checks.some(item=>item.id==="torrserver"&&item.ok),true);
    const playbackDiagnostics=await (await fetch(base+route+"/42/diagnostics")).json();assert.equal(playbackDiagnostics.checks.some(item=>item.id==="file"&&item.ok),true);
    assert.equal((await post("/api/maintenance/checkpoint",{})).status,200);
    async function file(){return (await (await fetch(base+route)).json()).files[0]}
    assert.equal((await file()).id,42);
    assert.equal((await file()).progress,12);
    assert.equal((await file()).resumeSeconds,123);
    const library=await (await fetch(base+"/api/library")).json();
    assert.equal(library.continueWatching[0].torrentHash,hash);
    assert.equal((await post(route+"/42/history",{action:"viewed",value:false})).status,200);
    assert.equal((await file()).viewed,false);
    assert.equal((await post(route+"/42/history",{action:"resetPosition"})).status,200);
    assert.equal((await file()).resumeSeconds,0);
    assert.equal((await file()).durationSeconds,1000);
    assert.equal((await file()).viewed,false);
    assert.equal((await post(route+"/42/history",{action:"viewed",value:"false"})).status,400);
    assert.equal((await post("/api/mpv/"+hash,{fileIndex:0})).status,400);
    assert.equal((await post("/api/mpv/"+hash,{fileIndex:999})).status,404);
    assert.deepEqual((await (await fetch(base+"/api/mpv/sessions")).json()).sessions,[]);
    const detail=await (await fetch(base+"/api/torrents/"+hash+"/details")).json();
    assert.equal(detail.item.torrentHash,hash);
    assert.equal(typeof detail.item.overview,"string");
    const added=await (await post("/api/torrents/add",{magnet:"magnet:?xt=urn:btih:"+"e".repeat(40)+"&dn=New%20Movie%202024"})).json();
    assert.equal(added.item.torrentHash,"e".repeat(40));
    assert.equal(added.item.title,"New Movie");
    assert.equal(added.item.year,2024);
    assert.equal(added.alreadyExists,false);
    const repeated=await (await post("/api/torrents/add",{magnet:"magnet:?xt=urn:btih:"+"e".repeat(40),title:"New Movie"})).json();
    assert.equal(repeated.alreadyExists,true);assert.equal(addedCount,1);
    assert.deepEqual((await (await fetch(base+"/api/mpv/sessions")).json()).sessions,[]);
  }finally{
    child.kill();await exited;
    await new Promise(r=>torr.close(r));
    rmSync(folder,{recursive:true,force:true});
  }
});
