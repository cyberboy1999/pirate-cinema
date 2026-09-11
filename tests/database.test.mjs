import test from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync,rmSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { MediaDatabase } from "../server/database.mjs";

test("persists media metadata and playback progress in SQLite",()=>{
  const root=mkdtempSync(join(tmpdir(),"pirate-cinema-db-test-"));
  try{
    const db=new MediaDatabase(join(root,"media.db"));
    db.upsert({torrentHash:"abc",torrentName:"Dune.Part.Two.2024.mkv",metadataProvider:"tmdb",providerId:"693134",title:"Dune: Part Two",year:2024,mediaType:"movie",genres:["Sci-Fi"],matchConfidence:.98,isActive:false});
    db.updateViewed("abc",{timecode:7200,duration:9960,lastPlayedAt:"2026-08-15T00:00:00.000Z"});
    const [item]=db.list();
    assert.equal(item.metadataProvider,"tmdb");
    assert.equal(item.providerId,"693134");
    assert.equal(item.progress,72);
    assert.equal(db.stats().continuing,1);
    db.close();
  }finally{rmSync(root,{recursive:true,force:true})}
});

test("updates an existing title after localization",()=>{
  const root=mkdtempSync(join(tmpdir(),"pirate-cinema-db-title-"));const database=new MediaDatabase(join(root,"media.db"));
  try{
    const base={torrentHash:"localized",torrentName:"Бивень / Tusk (2014)",year:2014,mediaType:"movie",genres:[]};
    database.upsert({...base,title:"Tusk",metadataProvider:"tvmaze",providerId:"1"});
    database.upsert({...base,title:"Бивень"});
    assert.equal(database.get("localized").title,"Бивень");
  }finally{database.close();rmSync(root,{recursive:true,force:true})}
});

test("remembers which torrent files have already been launched",()=>{
  const root=mkdtempSync(join(tmpdir(),"pirate-cinema-file-history-"));const database=new MediaDatabase(join(root,"media.db"));
  try{
    database.upsert({torrentHash:"series",torrentName:"Series",title:"Сериал",mediaType:"series",genres:[]});
    database.markFilePlayed("series",{fileIndex:3,fileName:"S01E03.mkv",filePath:"Season 1/S01E03.mkv"});
    database.markFilePlayed("series",{fileIndex:3,fileName:"S01E03.mkv",filePath:"Season 1/S01E03.mkv"});
    database.markFilePlayed("series",{fileIndex:4,fileName:"S01E04.mkv",filePath:"Season 1/S01E04.mkv"});
    const history=database.listFileHistory("series");
    assert.equal(history.length,2);
    assert.equal(history.find(item=>item.fileIndex===3).launchCount,2);
    assert.equal(history.find(item=>item.fileIndex===3).isWatched,1);
    database.updateFileProgress("series",3,{timecode:10,duration:100});
    assert.equal(database.listFileHistory("series").find(item=>item.fileIndex===3).isWatched,1);
    database.db.prepare("UPDATE media_file_history SET is_watched=0 WHERE torrent_hash=? AND file_index=?").run("series",3);
    database.db.prepare("DELETE FROM app_migrations WHERE id=?").run("viewed-on-launch-v1");
    database.migrate();
    assert.equal(database.listFileHistory("series").find(item=>item.fileIndex===3).isWatched,1);
    database.setFileViewed("series",3,false);
    database.updateFileProgress("series",3,{timecode:25,duration:100});
    database.migrate();
    assert.equal(database.listFileHistory("series").find(item=>item.fileIndex===3).isWatched,0);
    database.resetFileProgress("series",3);
    assert.equal(database.listFileHistory("series").find(item=>item.fileIndex===3).playbackTimecode,0);
    assert.equal(database.listFileHistory("series").find(item=>item.fileIndex===3).playbackDuration,100);
    assert.equal(database.listFileHistory("series").find(item=>item.fileIndex===3).isWatched,0);
    database.markFilePlayed("series",{fileIndex:3,fileName:"S01E03.mkv"});
    assert.equal(database.listFileHistory("series").find(item=>item.fileIndex===3).isWatched,1);
    database.remove("series");
    assert.deepEqual(database.listFileHistory("series"),[]);
  }finally{database.close();rmSync(root,{recursive:true,force:true})}
});

test("reconciles the library with TorrServer and removes a single card",()=>{
  const root=mkdtempSync(join(tmpdir(),"pirate-cinema-db-sync-"));const database=new MediaDatabase(join(root,"media.db"));
  try{
    for(const hash of ["keep","stale"])database.upsert({torrentHash:hash,torrentName:hash,title:hash,mediaType:"movie",genres:[]});
    assert.equal(database.reconcile(["keep"]),1);
    assert.deepEqual(database.list().map(item=>item.torrentHash),["keep"]);
    assert.equal(database.remove("keep"),1);
    assert.equal(database.stats().total,0);
  }finally{database.close();rmSync(root,{recursive:true,force:true})}
});
