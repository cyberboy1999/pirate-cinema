// Optional real-binary smoke check. Uses generated local video, never the user's library.
import assert from "node:assert/strict";
import {spawn,spawnSync} from "node:child_process";
import {mkdtempSync,mkdirSync,realpathSync,readdirSync,lstatSync,rmSync} from "node:fs";
import {resolve,join,sep} from "node:path";
import ffmpeg from "ffmpeg-static";
import {MediaDatabase} from "../server/database.mjs";
import {MpvPlayer} from "../server/mpv-player.mjs";

const work=resolve("work");mkdirSync(work,{recursive:true});
const folder=mkdtempSync(join(work,"mpv-ipc-check-"));
const video=join(folder,"local-test.mp4");
const generated=spawnSync(ffmpeg,["-hide_banner","-loglevel","error","-f","lavfi","-i","testsrc2=size=64x64:rate=5","-t","20","-c:v","libx264","-preset","ultrafast",video],{windowsHide:true});
assert.equal(generated.status,0,String(generated.stderr));
const db=new MediaDatabase(":memory:");
const files=[{id:42,path:video,name:"Локальный тест.mp4"},{id:87,path:video,name:"Следующий файл.mp4"}];
db.markFilePlayed("smoke",{fileIndex:42,fileName:files[0].name});
db.updateFileProgress("smoke",42,{timecode:5,duration:20});
const player=new MpvPlayer({db,client:{streamUrl:()=>video},executable:resolve("vendor/mpv/mpv.exe"),focusWindow:()=>{},
  spawnProcess:(exe,args)=>spawn(exe,[...args,"--no-config","--vo=null","--ao=null","--force-window=no","--pause=yes"],{windowsHide:true,stdio:"ignore"}),
});
async function until(check){
  const deadline=Date.now()+12000;
  while(!check()){if(Date.now()>deadline)throw new Error("Timed out: "+JSON.stringify(player.list()));await new Promise(r=>setTimeout(r,50))}
}
try{
  const opened=await player.play({hash:"smoke",files,fileIndex:42});
  await until(()=>player.list()[0]?.loading===false&&player.list()[0]?.duration>=19);
  assert.equal(db.listFileHistory("smoke")[0].isWatched,1);
  assert.equal(opened.resumeSeconds,5);
  const session=player.sessions.get(opened.id);
  assert.ok(Math.abs(Number(await player.command(session,["get_property","time-pos"]))-5)<1);
  await player.command(session,["seek",12,"absolute+exact"]);
  await until(()=>player.list()[0]?.timecode>=11.5);
  const duplicate=await player.play({hash:"smoke",files,fileIndex:42});
  assert.equal(duplicate.focused,true);assert.equal(player.list().length,1);
  await player.control(opened.id,"next");
  await until(()=>player.list()[0]?.loading===false&&player.list()[0]?.fileIndex===87);
  assert.ok(db.listFileHistory("smoke").find(h=>h.fileIndex===42).playbackTimecode>=11);
  assert.equal(db.listFileHistory("smoke").find(h=>h.fileIndex===87).isWatched,1);
  await player.control(opened.id,"stop");
  assert.equal(player.list().length,0);
  console.log("Real bundled MPV: IPC loadfile, resume, seek, duplicate focus, next file and SQLite persistence OK.");
}finally{
  player.close();db.close();
  // Delete only this run's inspected, non-linked generated fixture directory.
  const target=realpathSync(folder);
  const entries=readdirSync(target,{withFileTypes:true});
  assert.ok(target.startsWith(realpathSync(work)+sep)&&target!==realpathSync(work));
  assert.ok(!lstatSync(folder).isSymbolicLink()&&entries.every(e=>e.isFile()&&!e.isSymbolicLink()));
  rmSync(target,{recursive:true});
}
