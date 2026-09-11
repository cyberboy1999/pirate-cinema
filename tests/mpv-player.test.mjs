import test from "node:test";
import assert from "node:assert/strict";
import {EventEmitter} from "node:events";
import {MediaDatabase} from "../server/database.mjs";
import {MpvPlayer,sortMediaFiles,resumePosition} from "../server/mpv-player.mjs";

const files=[
  {id:19,path:"Show/S02/E01.mkv",name:"E01.mkv",season:2,episode:1},
  {id:8,path:"Show/S01/E10.mkv",name:"E10.mkv",season:1,episode:10},
  {id:42,path:"Show/S01/E02.mkv",name:"E02.mkv",season:1,episode:2},
];
function setup(){
  const db=new MediaDatabase(":memory:");
  db.upsert({torrentHash:"series",torrentName:"Show",title:"Show"});
  let spawned=0,focused=0;const commands=[];const sockets=[];
  const player=new MpvPlayer({db,client:{streamUrl:(hash,id)=>"http://localhost/stream?hash="+hash+"&index="+id},executable:"test-mpv",
    focusWindow:()=>{focused++},
    spawnProcess:()=>{spawned++;const child=new EventEmitter();child.pid=spawned;child.kill=()=>{child.killed=true};return child},
    connect:()=>{
      const socket=new EventEmitter();socket.destroyed=false;socket.setEncoding=()=>{};
      socket.destroy=()=>{socket.destroyed=true;socket.emit("close")};
      socket.event=event=>socket.emit("data",JSON.stringify(event)+"\n");
      socket.write=(line,callback)=>{
        const message=JSON.parse(line);commands.push(message.command);
        queueMicrotask(()=>{
          socket.event({request_id:message.request_id,error:"success"});
          if(message.command[0]==="loadfile"){
            socket.event({event:"end-file",reason:"stop"});
            socket.event({event:"start-file"});
            if(!socket.failLoad)socket.event({event:"file-loaded"});
            else socket.event({event:"end-file",reason:"error"});
          }
          callback?.();
        });
      };
      sockets.push(socket);queueMicrotask(()=>socket.emit("connect"));return socket;
    },
  });
  return {db,player,commands,sockets,get spawned(){return spawned},get focused(){return focused},close(){player.close();db.close()}};
}

test("media queue preserves true file IDs and naturally sorts seasons and episode names",()=>{
  assert.deepEqual(sortMediaFiles(files).map(f=>f.id),[42,8,19]);
  const collections=[{id:3,path:"B/2.mkv",name:"2.mkv"},{id:2,path:"A/10.mkv",name:"10.mkv"},{id:1,path:"A/2.mkv",name:"2.mkv"}];
  assert.deepEqual(sortMediaFiles(collections).map(f=>f.id),[1,2,3]);
  assert.equal(resumePosition({playbackTimecode:950,playbackDuration:1000}),950);
  assert.equal(resumePosition({playbackTimecode:950},"start"),0);
});

test("MPV IPC resumes, prevents duplicate windows, replaces explicitly and persists final position",async()=>{
  const f=setup();
  try{
    f.db.markFilePlayed("series",{fileIndex:42,fileName:"E02.mkv"});
    f.db.updateFileProgress("series",42,{timecode:120,duration:1000});
    const [first,second]=await Promise.all([f.player.play({hash:"series",files,fileIndex:42}),f.player.play({hash:"series",files,fileIndex:42})]);
    assert.equal(f.spawned,1);assert.equal(second.focused,true);assert.equal(f.focused,1);
    assert.equal(first.resumeSeconds,120);
    assert.equal(f.commands.find(c=>c[0]==="loadfile")[4].start,"120");
    assert.equal(f.db.listFileHistory("series")[0].launchCount,2);
    f.sockets[0].event({event:"property-change",name:"duration",data:1000});
    f.sockets[0].event({event:"property-change",name:"time-pos",data:140});
    await assert.rejects(f.player.play({hash:"series",files,fileIndex:8}),e=>e.status===409&&e.active[0].id===first.id);
    await f.player.play({hash:"series",files,fileIndex:8,mode:"start",existing:"replace",sessionId:first.id});
    assert.equal(f.spawned,1);
    assert.equal(f.db.listFileHistory("series").find(h=>h.fileIndex===42).playbackTimecode,140);
    assert.equal(f.player.list()[0].fileIndex,8);
    await f.player.play({hash:"series",files,fileIndex:19,existing:"new"});
    assert.equal(f.spawned,2);
    await assert.rejects(f.player.control(first.id,"next"),e=>e.status===409);
    assert.equal(f.player.list()[0].fileIndex,8);
    f.sockets[0].event({event:"property-change",name:"time-pos",data:55});
    await f.player.control(first.id,"stop");
    assert.equal(f.db.listFileHistory("series").find(h=>h.fileIndex===8).playbackTimecode,55);
    assert.equal(f.player.list().length,1);
  }finally{f.close()}
});

test("next file uses real index; auto-next only runs on EOF and failed streams are not marked viewed",async()=>{
  const f=setup();
  try{
    const first=await f.player.play({hash:"series",files,fileIndex:42,autoNext:true});
    f.sockets[0].event({event:"end-file",reason:"error"});await f.player.serial;
    assert.equal(f.player.list()[0].fileIndex,42);
    await f.player.control(first.id,"next");
    assert.equal(f.player.list()[0].fileIndex,8);
    f.sockets[0].event({event:"end-file",reason:"eof"});await f.player.serial;
    assert.equal(f.player.list()[0].fileIndex,19);
    assert.equal(f.commands.filter(c=>c[0]==="loadfile").at(-1)[1],"http://localhost/stream?hash=series&index=19");
    f.sockets[0].event({event:"end-file",reason:"eof"});await f.player.serial;
    assert.equal(f.spawned,1);assert.equal(f.player.list()[0].canNext,false);
    const bad={id:99,path:"bad.mkv",name:"bad.mkv"};
    f.sockets[0].failLoad=true;
    await f.player.play({hash:"series",files:[bad],fileIndex:99,existing:"replace",sessionId:first.id});
    assert.equal(f.db.listFileHistory("series").some(h=>h.fileIndex===99),false);
    assert.match(f.player.list()[0].error,/поток/);
    await assert.rejects(f.player.play({hash:"series",files,fileIndex:999}),e=>e.status===404);
  }finally{f.close()}
});
