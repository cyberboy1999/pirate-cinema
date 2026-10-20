// Exercise the actual embedded controller with deterministic media/network boundaries.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const root = path.join(__dirname, '..');
const source = fs.readFileSync(path.join(root, 'src/bin/dioxus.rs'), 'utf8');
let script = source.split('let script = format!(')[1].split('r#"')[1].split('"#,')[0];
for (const [key, value] of Object.entries({
  selector:'"torrent-1"', episodes:JSON.stringify([1,2,3].map(id => ({id,name:`S01E0${id}.mkv`}))),
  torrent_hash:'"torrent"',current_file_id:'1',next_episode_id:'2',audio_preference:'null',auto_next:'true',
  hls_js:'',artplayer_js:'',player_state_js:'',
})) script=script.replaceAll(`{${key}}`,value);
script=script.replaceAll('{{','{').replaceAll('}}','}');
class Node extends EventTarget {
  constructor() { super(); this.children=[]; this.style={}; this.dataset={}; this.hidden=false; this.value=''; }
  append(...items) { this.children.push(...items); }
  replaceChildren(...items) { this.children=items; this.value=items.find(item => item.selected)?.value ?? items[0]?.value ?? ''; }
  insertBefore(item,before) { this.children=this.children.filter(child=>child!==item); this.children.splice(this.children.indexOf(before),0,item); }
  setAttribute() {} scrollIntoView() {} focus() {} click() { this.dispatchEvent(new Event('click')); }
}
class Option { constructor(text,value,_,selected) { Object.assign(this,{text,value,selected}); } }
class Hls {
  static isSupported() { return true; }
  static Events={FRAG_BUFFERED:'buffer',FRAG_LOADED:'fragment',ERROR:'error',SUBTITLE_TRACKS_UPDATED:'subs'};
  static ErrorTypes={NETWORK_ERROR:'network',MEDIA_ERROR:'media'};
  constructor(options) { this.options=options; this.handlers={}; }
  on(event,handler) { this.handlers[event]=handler; }
  emit(event,value) { this.handlers[event]?.(event,value); }
  loadSource(url) { this.url=url; }
  attachMedia(video) { this.video=video; }
  destroy() { this.destroyed=true; }
  startLoad(position) { this.recoveredAt=position; }
  recoverMediaError() { this.mediaRecovered=true; }
}
let constructions=0;
class ArtPlayer {
  constructor(options) {
    constructions++; this.option=options; this.events={}; this.currentTime=0; this.duration=0; this.fullscreen=true;
    const video=new Node(); video.duration=0; video.currentTime=0;
    video.buffered={length:1,start:()=>0,end:()=>200};
    video.play=async()=>{this.playing=true;this.emit('video:play');};
    this.template={$video:video,$player:new Node(),$track:{track:{}}};
    this.setting={add:setting=>{this.subtitleSetting=setting;},update:setting=>{this.subtitleSetting=setting;}};
    this.subtitle={show:false,switch:async()=>{}}; this.notice={};
    this.url=options.url;
  }
  // Bundled ArtPlayer awaits before invoking customType; constructor returns first.
  set url(url) { this.source=url; setImmediate(()=>this.option.customType.m3u8(this.template.$video,url)); }
  get url() { return this.source; }
  on(event,fn) { (this.events[event]??=[]).push(fn); }
  emit(event,value) { for(const fn of this.events[event]??[]) fn(value); }
  pause() { this.playing=false; this.emit('video:pause'); }
  destroy() { this.destroyed=true; }
}
const container=new Node();
Object.assign(container.dataset,{hlsUrl:'http://127.0.0.1:8090/gst/torrent/master.m3u8?index=1&seconds=0',
  probeUrl:'http://127.0.0.1:8090/gst/torrent/probe?index=1',heartbeatUrl:'http://127.0.0.1:8090/gst/torrent/heartbeat'});
let attached=true;
const document=new Node(); Object.assign(document,{hidden:false,querySelector:()=>container,createElement:()=>new Node(),contains:()=>attached});
const window=new Node(); Object.assign(window,{Artplayer:ArtPlayer,Hls,__pirateCinemaVisible:true,focus:()=>{}});
const vm=require('node:vm');
vm.runInNewContext(fs.readFileSync(path.join(root,'assets/player-state.js'),'utf8'),{window});
const messages=[],fetches=[],waiting=[];
const timers=(fn,ms)=>ms===5000 ? (waiting.push(fn),0) : setTimeout(fn,ms);
const fetch=async url => {fetches.push(String(url));return {ok:true,json:async()=>({Tracks:[{Type:'audio',Index:0,Language:'rus',Title:'A'}]})};};
const run=new (Object.getPrototypeOf(async function(){}).constructor)('window','document','dioxus','fetch','Option','requestAnimationFrame','setTimeout',script);
const pending=run(window,document,{send:value=>messages.push(value)},fetch,Option,fn=>fn(),timers);
const tick=()=>new Promise(resolve=>setImmediate(resolve));
const metadata=art=>{art.duration=200;art.template.$video.duration=200;art.template.$video.dispatchEvent(new Event('loadedmetadata'));};
(async()=>{
  await tick(); const art=window.__pirateCinemaArt;
  assert.ok(art); assert.equal(window.__pirateCinemaSnapshot(),null,'loading must not overwrite resume');
  window.__pirateCinemaSave(); assert.equal(messages.filter(x=>x.kind==='progress').length,0);
  metadata(art); art.currentTime=100; art.template.$video.currentTime=100;
  art.emit('video:pause'); assert.equal(messages.at(-1).position,100);
  const player=art.template.$player;
  const next=player.children.find(node=>node.className==='pc-next-episode');
  art.template.$video.dispatchEvent(new Event('timeupdate'));
  assert.equal(next.style.display,'block','next episode visible in final two minutes');
  const controls=player.children.find(node=>node.className==='pc-player-topbar');
  assert.ok(controls.children.some(node=>node.className==='pc-episode'));
  assert.ok(controls.children.some(node=>node.className==='pc-audio'));
  art.emit('fullscreen',true);assert.equal(messages.at(-1).kind,'fullscreen');
  assert.equal(messages.at(-1).active,true);
  assert.ok(fetches.some(url=>url.includes('probe?index=2')),'bounded next-file preparation');
  next.click(); assert.equal(messages.at(-1).file_id,2);
  for(const id of [2,3]) {
    container.dataset.hlsUrl=`http://127.0.0.1:8090/gst/torrent/master.m3u8?index=${id}&seconds=0`;
    container.dataset.probeUrl=`http://127.0.0.1:8090/gst/torrent/probe?index=${id}`;
    const change=window.__pirateCinemaSwitch(id,id===3?null:3);
    await tick(); metadata(art); await change; await tick();
    assert.equal(window.__pirateCinemaArt,art); assert.equal(art.fullscreen,true);
    assert.equal(constructions,1); assert.equal(window.__pirateCinemaSnapshot().file_id,id);
  }
  const top=player.children.find(node=>node.className==='pc-player-topbar');
  const auto=top.children.find(node=>node.className==='pc-auto-next').children[0];
  auto.checked=false;auto.dispatchEvent(new Event('change'));assert.equal(messages.at(-1).active,false);
  art.emit('video:ended');assert.notEqual(messages.at(-1).kind,'ended');
  auto.checked=true;auto.dispatchEvent(new Event('change'));
  window.__pirateCinemaVisible=false; art.template.$video.play(); assert.equal(art.playing,false);
  const before=messages.filter(x=>x.kind==='ended').length;
  art.emit('video:ended');assert.equal(messages.filter(x=>x.kind==='ended').length,before);
  window.__pirateCinemaVisible=true;window.dispatchEvent(new Event('pc-visibility'));
  assert.equal(messages.at(-1).kind,'ended');assert.equal(messages.at(-1).source_file_id,3);
  art.currentTime=87; art.template.$video.currentTime=87;
  const hls=window.__pirateCinemaHls;
  hls.emit('error',{fatal:true,type:'network'});assert.equal(hls.recoveredAt,87);
  for(let i=0;i<3;i++) hls.emit('error',{fatal:true,type:'network',details:'offline'});
  const status=player.children.find(node=>node.className==='pc-stream-status');
  assert.equal(status.children[1].hidden,false);status.children[1].click();
  assert.ok(window.__pirateCinemaHls.url.includes('seconds=0')); // HLS uses startPosition, not a second server seek.
  assert.equal(window.__pirateCinemaHls.options.startPosition,87);
  assert.equal(constructions,1);assert.equal(art.fullscreen,true);
  attached=false;waiting.shift()();await pending;
  console.log('Controller: repeated episodes, history, visibility, auto-next, recovery and fullscreen: OK');
})().catch(error=>{console.error(error);attached=false;waiting.shift()?.();process.exitCode=1;});
