import {useEffect,useRef,useState} from "react";
import Artplayer from "artplayer";
import Hls from "hls.js";
import artplayerPluginHlsControl from "artplayer-plugin-hls-control";
import {Capacitor,CapacitorHttp,registerPlugin} from "@capacitor/core";

const TorrentMedia=registerPlugin<{prepare(options:{url:string;token:string}):Promise<{url:string;token:string}>;stop(options:{token:string}):Promise<void>}>("TorrentMedia");

type Props={url:string;title:string;position?:number;rutubeId?:string;onProgress?:(position:number,duration:number,completed:boolean)=>void};
export default function EmbeddedPlayer({url,title,position=0,rutubeId,onProgress}:Props){
  const root=useRef<HTMLDivElement>(null),progress=useRef(onProgress),[error,setError]=useState("");
  useEffect(()=>{progress.current=onProgress},[onProgress]);
  useEffect(()=>{
    let cancelled=false,completedPlayback=false,reportedComplete=false,nativeToken="",art:Artplayer|undefined,hls:Hls|undefined,wakeLock:WakeLockSentinel|undefined,lastFlush=0,persist=()=>{};
    queueMicrotask(()=>{if(!cancelled)setError("")});
    async function start(){
      let source=url;
      if(rutubeId){
        if(!/^[a-f\d]{32}$/.test(rutubeId))throw new Error("Invalid RuTube ID");
        const response=await CapacitorHttp.get({url:`https://rutube.ru/api/play/options/${rutubeId}/?format=json`,responseType:"json",connectTimeout:10000,readTimeout:15000});
        if(response.status!==200)throw new Error(`RuTube HTTP ${response.status}`);
        const data=typeof response.data==="string"?JSON.parse(response.data):response.data;
        source=data.video_balancer?.m3u8;
        if(typeof source!=="string"||new URL(source).protocol!=="https:")throw new Error("RuTube не предоставил доступный HLS-поток");
      }else if(Capacitor.isNativePlatform()&&/^http:\/\/127\.0\.0\.1:3030\/torrents\//.test(url)){
        nativeToken=crypto.randomUUID().replaceAll("-","");
        const prepared=await TorrentMedia.prepare({url,token:nativeToken});
        nativeToken=prepared.token;source=prepared.url;
        if(cancelled){await TorrentMedia.stop({token:nativeToken});return}
      }
      if(cancelled||!root.current)return;
      const isHls=Boolean(rutubeId)||/\.m3u8(?:\?|$)/i.test(source);
      art=new Artplayer({container:root.current,url:source,autoplay:true,playsInline:true,setting:true,fullscreen:true,fullscreenWeb:true,pip:true,playbackRate:true,aspectRatio:true,hotkey:true,type:isHls?"m3u8":"",plugins:isHls?[artplayerPluginHlsControl({quality:{setting:true,control:true},audio:{setting:true,control:true}})]:[],customType:{m3u8(video,stream,instance){
        if(Hls.isSupported()){hls=new Hls({maxBufferLength:30,maxMaxBufferLength:60,xhrSetup(xhr,requestUrl){/* Capacitor reports its interceptor URL; HLS needs the original base for relative segments. */Object.defineProperty(xhr,"responseURL",{configurable:true,get:()=>requestUrl})}});Object.assign(instance,{hls});hls.loadSource(stream);hls.attachMedia(video);hls.on(Hls.Events.ERROR,(_event,data)=>{console.warn("HLS load error",data.details,data.response?.code,data.error?.message);if(data.fatal)setError(`HLS: ${data.details} HTTP ${data.response?.code??"network"}`)})}
        else if(video.canPlayType("application/vnd.apple.mpegurl"))video.src=stream;
        else setError("HLS не поддерживается устройством");
      }}});
      let restored=false;
      art.on("video:loadedmetadata",()=>{
        if(!restored&&art){restored=true;if(position>0)art.currentTime=position}
        if(!isHls&&art){
          const tracks=Reflect.get(art.video,"audioTracks") as {length:number;[index:number]:{label:string;language:string;enabled:boolean}}|undefined;
          if(tracks&&tracks.length>1)art.setting.add({name:"audio-track",html:"Звуковая дорожка",selector:Array.from({length:tracks.length},(_,index)=>({html:tracks[index].label||tracks[index].language||`Дорожка ${index+1}`,value:index,default:tracks[index].enabled})),onSelect(item){const selected=Number(item.value);for(let index=0;index<tracks.length;index++)tracks[index].enabled=index===selected;return item.html}});
        }
      });
      const flush=(completed=false,force=false)=>{completedPlayback=completedPlayback||completed;if(reportedComplete||!force&&!completed&&Date.now()-lastFlush<5000)return;if(art&&Number.isFinite(art.duration)&&art.duration>0&&art.video.readyState>0){lastFlush=Date.now();progress.current?.(completedPlayback?0:art.currentTime,art.duration,completedPlayback);reportedComplete=completedPlayback}};
      persist=()=>flush(false,true);
      art.on("video:timeupdate",()=>flush());art.on("video:pause",()=>{flush(false,true);void wakeLock?.release()});art.on("video:ended",()=>flush(true));
      art.on("video:playing",()=>{if("wakeLock" in navigator&&(!wakeLock||wakeLock.released))void navigator.wakeLock.request("screen").then(lock=>{if(cancelled)void lock.release();else wakeLock=lock}).catch(()=>{})});
      art.on("video:error",()=>setError("Браузер не может декодировать поток. Для MKV/неподдерживаемого аудио требуется remux или транскодирование."));
      art.on("destroy",()=>hls?.destroy());
    }
    void start().catch(reason=>{if(!cancelled)setError(reason instanceof Error?reason.message:"Ошибка воспроизведения")});
    return()=>{cancelled=true;persist();void wakeLock?.release();art?.destroy(false);hls?.destroy();if(nativeToken)void TorrentMedia.stop({token:nativeToken}).catch(()=>{})};
  },[url,title,position,rutubeId]);
  return <section aria-label="Artplayer"><div ref={root} style={{width:"100%",aspectRatio:"16/9",minHeight:200,background:"#000"}}/>{error&&<p role="alert">{error}</p>}</section>;
}
