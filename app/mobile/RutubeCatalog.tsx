import {useEffect,useState} from "react";
import {CapacitorHttp} from "@capacitor/core";
import {rutubeVideo,savedRutubeVideos} from "./rutube.mjs";
import styles from "./mobile.module.css";
export type RutubeItem={id:string;url:string;title:string;position:number;duration:number;completed:boolean};
export default function RutubeCatalog({onPlay,english}:{onPlay:(videos:RutubeItem[])=>void;english:boolean}){
  const [saved,setSaved]=useState<{url:string;videos:RutubeItem[]}[]>([]);
  useEffect(()=>{try{const raw=JSON.parse(localStorage.getItem("pirate-cinema-mobile-rutube-playlists")||"[]");if(Array.isArray(raw))queueMicrotask(()=>setSaved(raw.filter(item=>typeof item?.url==="string"&&/^https:\/\/rutube\.ru\/plst\/\d+\/$/.test(item.url)).map(item=>({url:item.url,videos:savedRutubeVideos(item.videos)}))))}catch{/* Saved videos remain available separately. */}},[]);
  function remember(){const match=query.trim().match(/\/plst\/(\d+)/);if(!match)return;const url=`https://rutube.ru/plst/${match[1]}/`,next=[{url,videos:results},...saved.filter(item=>item.url!==url)];localStorage.setItem("pirate-cinema-mobile-rutube-playlists",JSON.stringify(next));setSaved(next);onPlay(results)}
  const [query,setQuery]=useState(""),[results,setResults]=useState<RutubeItem[]>([]),[busy,setBusy]=useState(false),[error,setError]=useState(""),[next,setNext]=useState<string|null>(null),[playlist,setPlaylist]=useState(false);
  async function load(more=false){setBusy(true);setError("");try{
    let url:string;
    const match=query.trim().match(/^https:\/\/(?:www\.)?rutube\.ru\/plst\/(\d+)\/?(?:\?.*)?$/);
    const isPlaylist=Boolean(match);setPlaylist(isPlaylist);
    if(more&&next)url=next;else url=isPlaylist?`https://rutube.ru/api/playlist/custom/${match![1]}/videos/?format=json`:`https://rutube.ru/api/search/video/?query=${encodeURIComponent(query.trim())}&format=json`;
    const target=new URL(url);if(target.origin!=="https://rutube.ru"||!target.pathname.startsWith("/api/"))throw new Error("Invalid RuTube API URL");
    const response=await CapacitorHttp.get({url:target.href,connectTimeout:10000,readTimeout:15000,responseType:"json"});
    if(response.status!==200)throw new Error(`RuTube HTTP ${response.status}`);
    const data=typeof response.data==="string"?JSON.parse(response.data):response.data;
    if(!Array.isArray(data.results))throw new Error("Invalid RuTube response");
    const videos=data.results.flatMap((raw:{video_url:string;title:string;duration:number})=>{try{return [{...rutubeVideo(raw.video_url),title:String(raw.title||"RuTube"),position:0,duration:Number(raw.duration)||0,completed:false}]}catch{return []}});
    setResults(current=>more?[...current,...videos.filter((item:RutubeItem)=>!current.some(old=>old.id===item.id))]:videos);setNext(data.has_next&&typeof data.next==="string"?new URL(data.next,"https://rutube.ru").href:null);
  }catch(reason){setError(reason instanceof Error?reason.message:"RuTube unavailable")}finally{setBusy(false)}}
return <section>{saved.map(item=><div key={item.url}><button onClick={()=>onPlay(item.videos)}>{english?"Saved playlist":"Сохранённый плейлист"} {item.url.match(/\d+/)?.[0]} ({item.videos.length})</button><button aria-label={english?"Delete playlist":"Удалить плейлист"} onClick={()=>{const next=saved.filter(value=>value.url!==item.url);setSaved(next);localStorage.setItem("pirate-cinema-mobile-rutube-playlists",JSON.stringify(next))}}>×</button></div>)}<form onSubmit={event=>{event.preventDefault();void load()}}><label>{english?"Search or playlist URL":"Поиск или ссылка на плейлист"}<input value={query} onChange={event=>setQuery(event.target.value)} placeholder="https://rutube.ru/plst/…"/></label><button disabled={busy||query.trim().length<2} type="submit">{busy?(english?"Loading…":"Загрузка…"):(english?"Find / open playlist":"Найти / открыть плейлист")}</button></form>{error&&<p role="alert">{error}</p>}{playlist&&results.length>0&&<button onClick={remember}>{english?"Save and play playlist":"Сохранить и воспроизвести плейлист"} ({results.length})</button>}<div className={styles.list}>{results.map((video,index)=><article key={video.id}><button className={styles.info} onClick={()=>onPlay(playlist?results.slice(index):[video])}><strong>{video.title}</strong><small>{Math.floor(video.duration/60)} min</small></button></article>)}</div>{next&&<button disabled={busy} onClick={()=>void load(true)}>{english?"Load more":"Загрузить ещё"}</button>}</section>;
}
