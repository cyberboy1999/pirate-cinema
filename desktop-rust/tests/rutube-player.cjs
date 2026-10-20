const assert = require('node:assert/strict');
const vm = require('node:vm');
const fs = require('node:fs');
let options, loader, hls, quality, episodes;
const settings = {};
let playerInstances = 0;
const sent = [];
class DefaultLoader {
    constructor() { this.stats = {loading:{}, loaded:0, total:0}; }
    abort() {}
    destroy() {}
}
class Hls {
    static DefaultConfig = {loader:DefaultLoader};
    static Events = {MANIFEST_PARSED:'manifest', ERROR:'error', LEVELS_UPDATED:'levels'};
    static isSupported() { return true; }
    constructor(config) { hls = this; loader = new config.loader(); this.handlers = {}; this.levels = [{height:360,bitrate:500000},{height:1080,bitrate:3000000},{height:720,bitrate:1500000}]; }
    loadSource(url) { this.url = url; }
    attachMedia() {}
    stopLoad() {}
    recoverMediaError() {}
    on(event,callback) { this.handlers[event] = callback; }
}
class Artplayer {
    constructor(config) { playerInstances++; options = config; this.handlers = {}; this.currentTime = 123; this.paused = true; this.fullscreen = true; this.controls={add:value=>{this.controls[value.name]={...value,style:{...value.style}};}}; const save = value => { settings[value.name]=value; if(value.name === 'rutube-episodes') episodes=value; if(value.name === 'rutube-quality') quality=value; }; this.setting = {add:save,update:save}; config.customType.m3u8({},config.url,this); }
    on(name,callback) { this.handlers[name] = callback; }
    pause() { this.paused = true; }
    play() { this.paused = false; return Promise.resolve(); }
    destroy() { this.handlers.destroy?.(); }
}
const context = {window:{},Hls,Artplayer,Uint8Array,TextDecoder,performance,setTimeout,clearTimeout,atob,
    dioxus:{send:event => sent.push(event)},document:{getElementById:() => ({scrollIntoView(){}})}};
vm.runInNewContext(fs.readFileSync(require('node:path').join(__dirname,'../assets/rutube-player.js'),'utf8'),context);
context.window.startRutube({url:'https://bl.rutube.ru/test.m3u8',token:7});
hls.handlers.manifest();
assert.deepEqual(Array.from(quality.selector,item=>item.html),['Авто','1080p','720p','360p']);
assert.equal(quality.onSelect(quality.selector[1]),'1080p');
assert.equal(hls.nextLevel,1); // Sorted label must still target the original manifest index.
assert.equal(quality.onSelect(quality.selector[0]),'Авто');
assert.equal(hls.nextLevel,-1);
quality.onSelect({value:999});
assert.equal(hls.nextLevel,-1);
assert.equal(context.window.rutubePlayer.currentTime,123);
assert.equal(context.window.rutubePlayer.paused,true);
assert.equal(context.window.rutubePlayer.fullscreen,true);
assert.equal(playerInstances,1);
hls.levels = [];
hls.handlers.levels();
assert.equal(quality.selector.length,1);
sent.length = 0;
console.log('PASS: quality auto/manual, real level indexes, invalid/empty levels, retained player state');
const request = {url:'https://bl.rutube.ru/test.m3u8',responseType:'text',rangeStart:0,rangeEnd:4};
loader.load(request,{}, {onSuccess(response) {assert.equal(response.data,'test');},onError(){throw Error('unexpected error');}});
assert.equal(sent[0].start,0); assert.equal(sent[0].end,4);
context.window.rutubeReply(7,sent[0].id,Buffer.from('test').toString('base64'),'');
loader.load(request,{}, {onSuccess(){throw Error('aborted request completed');}});
loader.abort();
context.window.rutubeReply(7,sent[1].id,Buffer.from('test').toString('base64'),'');
context.window.rutubePlayer.handlers.destroy();
assert.equal(sent.at(-1).closed,true);
loader.load({...request,rangeStart:0,rangeEnd:0},{}, {onSuccess(){},onError(){}});
assert.equal(sent.at(-1).start,null);
assert.equal(sent.at(-1).end,null);
loader.abort();
assert.equal(options.autoplay,false);
console.log('PASS: native HLS response, ranges, abort, close and no autoplay');
const source = fs.readFileSync(require('node:path').join(__dirname,'../src/bin/dioxus.rs'),'utf8');
const expression = source.match(/const canStart = \(\) => (.*);/)[1];
const gate = {window:{__pirateCinemaVisible:true,__pirateCinemaRutubeActive:false},document:{hidden:false}};
vm.runInNewContext(fs.readFileSync(require('node:path').join(__dirname,'../assets/player-state.js'),'utf8'),gate);
assert.equal(vm.runInNewContext(expression,gate),true);
gate.window.__pirateCinemaRutubeActive = true;
assert.equal(vm.runInNewContext(expression,gate),false);
console.log('PASS: torrent startup is blocked while Rutube is active');
context.window.startRutube({url:'https://bl.rutube.ru/first.m3u8',token:8,id:'first',queue:[{id:'first',title:'<img onerror="bad">'},{id:'second',title:'Второй'}]});
assert.equal(episodes.selector[0].html.includes('<img'),false);
const player = context.window.rutubePlayer;
const count = playerInstances;
sent.length=0;
context.document.hidden=true;
player.handlers['video:ended']();
assert.equal(sent.length,0);
context.document.hidden=false;
player.handlers['video:ended']();
assert.equal(sent.at(-1).episode,'second');
player.handlers['video:ended']();
assert.equal(sent.length,1); // Duplicate ended while resolving is ignored.
context.window.rutubeSwitch({id:'second',url:'https://bl.rutube.ru/second.m3u8'});
assert.equal(hls.url,'https://bl.rutube.ru/second.m3u8');
assert.equal(playerInstances,count);
assert.equal(player.fullscreen,true);
assert.equal(player.currentTime,0);
hls.handlers.manifest();
assert.equal(player.paused,false);
sent.length=0;
player.handlers['video:ended']();
assert.equal(sent.length,0); // Last episode does not wrap.
episodes.onSelect({value:'forged'});
assert.equal(sent.length,0);
episodes.onSelect({value:'first'});
assert.equal(sent.at(-1).episode,'first');
context.window.__pirateCinemaVisible=false;
context.window.rutubeSwitch({id:'first',url:'https://bl.rutube.ru/first.m3u8'});
hls.handlers.manifest();
assert.equal(player.paused,true);
console.log('PASS: playlist selection, next/last, duplicate events, same player/fullscreen and hidden playback guard');
context.window.startRutube({url:'https://bl.rutube.ru/resume.m3u8',token:9,id:'saved',position:42});
const saved = context.window.rutubePlayer;
saved.duration=120;
saved.handlers['video:loadedmetadata']();
assert.equal(saved.currentTime,42);
sent.length=0;
saved.handlers['video:playing']();
assert.equal(sent.find(event=>event.progress)?.position,42);
saved.currentTime=56;
saved.handlers['video:pause']();
assert.equal(sent.at(-1).position,56);
context.window.rutubeFlush();
saved.destroy();
assert.equal(sent.at(-2).progress,'saved');
assert.equal(sent.at(-1).closed,true);
console.log('PASS: resume position and progress flush on play/pause/destroy');
context.window.__pirateCinemaVisible=true;
context.window.startRutube({url:'https://bl.rutube.ru/season.m3u8',token:10,id:'one',quality:720,autoNext:false,queue:[{id:'one',title:'Название – 1 сезон 1 серия'},{id:'two',title:'Название – 1 сезон 2 серия'},{id:'three',title:'Название S02E01'}]});
const improved=context.window.rutubePlayer;
improved.duration=600;
hls.handlers.manifest();
assert.equal(hls.nextLevel,2);
assert.deepEqual(Array.from(episodes.selector,item=>item.html),['Серия 1','Серия 2']);
settings['rutube-season'].onSelect({value:2,html:'Сезон 2'});
assert.equal(episodes.selector.length,1);
assert.equal(episodes.selector[0].value,'three');
improved.currentTime=479;
improved.handlers['video:timeupdate']();
assert.equal(improved.controls['rutube-next'].style.display,'none');
improved.currentTime=480;
improved.handlers['video:timeupdate']();
assert.equal(improved.controls['rutube-next'].style.display,'');
sent.length=0;
improved.handlers['video:ended']();
assert.equal(sent.some(event=>event.episode),false);
settings['rutube-auto-next'].onSwitch({switch:false});
assert.equal(sent.at(-1).value,1);
improved.controls['rutube-next'].click();
assert.equal(sent.at(-1).episode,'two');
context.window.rutubeSwitch({id:'two',url:'https://bl.rutube.ru/two.m3u8'});
hls.levels=[{height:720},{height:360}];
hls.handlers.manifest();
assert.equal(hls.nextLevel,0); // Resolution, not prior manifest index.
hls.levels=[{height:360}];
hls.handlers.manifest();
assert.equal(hls.nextLevel,-1);
improved.currentTime=120; improved.paused=false;
improved.controls['rutube-retry'].click();
assert.equal(sent.at(-1).retry,'two');
context.window.rutubeRecover('https://bl.rutube.ru/renewed.m3u8');
improved.handlers['video:loadedmetadata']();
assert.equal(improved.currentTime,120);
hls.handlers.manifest();
assert.equal(improved.paused,false);
assert.equal(improved.fullscreen,true);
improved.destroy();
console.log('PASS: compact seasons, persistent resolution, next countdown/toggle and recovery position/fullscreen');
context.window.startRutube({url:'https://bl.rutube.ru/pending.m3u8',token:11,id:'pending',position:42});
const pendingResume=context.window.rutubePlayer;
pendingResume.currentTime=0; pendingResume.duration=120;
pendingResume.controls['rutube-retry'].click();
context.window.rutubeRecover('https://bl.rutube.ru/retry-pending.m3u8');
pendingResume.handlers['video:loadedmetadata']();
assert.equal(pendingResume.currentTime,42);
pendingResume.destroy();
console.log('PASS: recovery before metadata retains saved resume position');
const scheduled=[];
context.setTimeout=(callback,delay)=>{const timer={callback,delay,cancelled:false};scheduled.push(timer);return timer;};
context.clearTimeout=timer=>{if(timer)timer.cancelled=true;};
context.window.startRutube({url:'https://bl.rutube.ru/failure.m3u8',token:12,id:'failure',position:42});
const failing=context.window.rutubePlayer;
failing.duration=120;failing.currentTime=0;
for(let attempt=0;attempt<2;attempt++) {
    hls.handlers.error(null,{fatal:true,type:'networkError'});
    assert.equal(scheduled.at(-1).delay,(attempt+1)*2000);
    scheduled.at(-1).callback();
    assert.equal(sent.at(-1).retry,'failure');
    context.window.rutubeRecover('https://bl.rutube.ru/fresh.m3u8');
}
hls.handlers.error(null,{fatal:true,type:'networkError'});
assert.equal(scheduled.length,2);
failing.handlers['video:loadedmetadata']();
assert.equal(failing.currentTime,42);
failing.destroy();
assert.equal(scheduled.at(-1).cancelled,true);
console.log('PASS: bounded automatic retries, pending position and timer cleanup');
