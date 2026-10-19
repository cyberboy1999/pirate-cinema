const VIDEO=/\.(?:mkv|mp4|avi|mov|m4v|webm|ts|m2ts)$/i;
import {savedRutubeVideos} from "./rutube.mjs";
const NATURAL=new Intl.Collator(undefined,{numeric:true,sensitivity:"base"});

export function episodeNumbers(value){
  const text=String(value??"");const match=text.match(/\bS(\d{1,2})E(\d{1,3})\b/i)??text.match(/\b(\d{1,2})[xх](\d{1,3})\b/i);if(match)return {season:Number(match[1]),episode:Number(match[2])};
  const season=Number(text.match(/(?:season|сезон)[ ._-]*(\d{1,2})/i)?.[1])||null,episode=Number(text.match(/(?:episode|эпизод|серия)[ ._-]*(\d{1,3})/i)?.[1])||null;return {season,episode};
}

export function sortEpisodeFiles(files){return [...files].sort((a,b)=>(a.season??999)-(b.season??999)||(a.episode??9999)-(b.episode??9999)||NATURAL.compare(a.path,b.path))}

export function nextUnviewedFile(files,history,hash,currentId){
  const sorted=sortEpisodeFiles(files),start=sorted.findIndex(file=>file.id===currentId),candidates=start<0?sorted:[...sorted.slice(start+1),...sorted.slice(0,start)];return candidates.find(file=>!history?.[`${hash}:${file.id}`]?.completed)??null;
}

export function magnetInfoHash(value){
  let url,hash="";try{url=new URL(String(value));if(url.protocol!=="magnet:")return null;hash=url.searchParams.getAll("xt").map(item=>item.match(/^urn:btih:(.+)$/i)?.[1]??"").find(Boolean)??""}catch{return null}
  if(/^[a-f\d]{40}$/i.test(hash))return hash.toLowerCase();
  if(!/^[a-z2-7]{32}$/i.test(hash))return null;
  let bits="";for(const character of hash.toUpperCase()){const value="ABCDEFGHIJKLMNOPQRSTUVWXYZ234567".indexOf(character);bits+=value.toString(2).padStart(5,"0")}
  return Array.from({length:20},(_,index)=>Number.parseInt(bits.slice(index*8,index*8+8),2).toString(16).padStart(2,"0")).join("");
}

export function newerVersion(candidate,current){
  const parse=value=>String(value).replace(/^(?:android-)?v/,"").match(/^(\d+)\.(\d+)\.(\d+)(?:-([\da-z.-]+))?$/i);
  const a=parse(candidate),b=parse(current);if(!a||!b)return false;
  for(let index=1;index<=3;index++)if(Number(a[index])!==Number(b[index]))return Number(a[index])>Number(b[index]);
  if(!a[4]||!b[4])return !a[4]&&Boolean(b[4]);
  const left=a[4].split("."),right=b[4].split(".");for(let index=0;index<Math.max(left.length,right.length);index++){if(left[index]===right[index])continue;if(left[index]===undefined)return false;if(right[index]===undefined)return true;const x=/^\d+$/.test(left[index]),y=/^\d+$/.test(right[index]);return x&&y?Number(left[index])>Number(right[index]):x!==y?!x:left[index]>right[index]}return false;
}

export function androidRelease(releases,current){
  if(!Array.isArray(releases))return null;
  return releases.filter(release=>!release.draft&&(!release.prerelease||current.includes("-"))).map(release=>({release,asset:release.assets?.find(asset=>/^Pirate-Cinema-Android-[\da-z.-]+-arm64-v8a\.apk$/i.test(asset.name??"")&&typeof asset.browser_download_url==="string"&&asset.browser_download_url.startsWith("https://github.com/cyberboy1999/pirate-cinema/releases/download/"))})).find(item=>item.asset&&newerVersion(item.release.tag_name,current))??null;
}

export function serverCandidates(current,subnet){
  const local=[current,"http://127.0.0.1:8091","http://127.0.0.1:8090","http://10.0.2.2:8091","http://10.0.2.2:8090"];
  const lan=subnet?Array.from({length:254},(_,index)=>`http://${subnet}${index+1}:8090`):[];
  return [...new Set([...local,...lan].filter(Boolean))];
}

export async function probeTorrServer(candidates,request=fetch){
  // ponytail: bounded batches keep a /24 scan usable on phones; add native discovery only if this misses real servers.
  for(let offset=0;offset<candidates.length;offset+=16){
    try{return await Promise.any(candidates.slice(offset,offset+16).map(async base=>{
      const response=await request(`${base}/echo`,{signal:AbortSignal.timeout(900)});
      if(!response.ok)throw new Error("TorrServer unavailable");
      return base;
    }))}catch{/* Try the next batch. */}
  }
  return null;
}

export function metadataQuery(value){
  return String(value??"").replace(/[._]+/g," ").replace(/\b(?:19|20)\d{2}\b.*$/i,match=>match.slice(0,4)).replace(/\b(?:S\d{1,2}(?:E\d{1,3})?|UHD|BDRip|WEB[- ]?DL|BluRay|REMUX|1080p|2160p|720p|4K|HDR|SDR|x26[45]).*$/i,"").replace(/\s+/g," ").trim();
}

export function normalizeCinemeta(raw){
  const year=Number(String(raw?.year??raw?.releaseInfo??raw?.released??"").match(/(?:19|20)\d{2}/)?.[0])||null;
  return {id:String(raw?.imdb_id??raw?.id??""),title:String(raw?.name??raw?.title??""),year,poster:String(raw?.poster??""),backdrop:String(raw?.background??""),description:String(raw?.description??""),rating:Number(raw?.imdbRating)||null,genres:Array.isArray(raw?.genres)?raw.genres:[]};
}

export function enableRutorSearch(settings){return settings?.EnableRutorSearch?null:{...(settings??{}),EnableRutorSearch:true}}

export function playbackHistoryEntry(previous,result,media,now=new Date().toISOString()){
  if(!result?.ok&&!Number.isFinite(result?.positionMs))return previous??null;
  const completed=Boolean(result.completed),durationMs=Math.max(0,Math.floor(Number(result.durationMs)||Number(previous?.durationMs)||0));
  return {...previous,...media,positionMs:completed?0:Math.max(0,Math.floor(Number(result.positionMs)||0)),durationMs,completed,lastPlayedAt:now};
}

export function updatePlaybackHistory(history,key,changes){
  const current=history?.[key];
  if(!current)return history??{};
  return {...history,[key]:{...current,...changes}};
}

export function removeTorrentHistory(history,hash){
  return Object.fromEntries(Object.entries(history??{}).filter(([,entry])=>entry?.hash!==hash));
}

export function createMobileBackup({serverUrl,player,history,metadata,rutube=[],rutubePlaylists=[],rutubeQueue=[]},createdAt=new Date().toISOString()){
  return JSON.stringify({schema:2,createdAt,serverUrl,player,history:history??{},metadata:metadata??{},rutube,rutubePlaylists,rutubeQueue},null,2);
}

const isRecord=value=>value!==null&&typeof value==="object"&&!Array.isArray(value);
export function writeStorageTransaction(storage,changes){
  const previous=Object.fromEntries(Object.keys(changes).map(key=>[key,storage.getItem(key)]));
  try{for(const [key,value] of Object.entries(changes))storage.setItem(key,value)}
  catch(error){
    // Remove staged values first: quota rollback must not need additional space.
    for(const key of Object.keys(changes))storage.removeItem(key);
    for(const [key,value] of Object.entries(previous))if(value!==null)storage.setItem(key,value);
    throw error;
  }
}
export function safeMobileHistory(value){
  if(!isRecord(value))return {};
  return Object.fromEntries(Object.entries(value).filter(([key,item])=>isRecord(item)&&/^[a-f\d]{40}$/i.test(item.hash)&&Number.isSafeInteger(item.fileId)&&item.fileId>=0&&key===`${item.hash}:${item.fileId}`&&typeof item.fileName==="string"&&typeof item.title==="string"&&Number.isFinite(item.positionMs)&&item.positionMs>=0&&Number.isFinite(item.durationMs)&&item.durationMs>=0&&typeof item.completed==="boolean"&&typeof item.lastPlayedAt==="string"&&Number.isFinite(Date.parse(item.lastPlayedAt))));
}
export function safeMobileMetadata(value){
  if(!isRecord(value))return {};
  return Object.fromEntries(Object.entries(value).filter(([key,item])=>/^[a-f\d]{40}$/i.test(key)&&isRecord(item)&&["id","title","poster","backdrop","description"].every(field=>typeof item[field]==="string")&&(item.year===null||Number.isInteger(item.year))&&(item.rating===null||Number.isFinite(item.rating))&&Array.isArray(item.genres)&&item.genres.every(genre=>typeof genre==="string")));
}

export function parseMobileBackup(content){
  if(typeof content!=="string"||new TextEncoder().encode(content).length>5_000_000)throw new Error("Резервная копия слишком большая");
  const data=JSON.parse(content),record=value=>value!==null&&typeof value==="object"&&!Array.isArray(value);
  if(![1,2].includes(data?.schema)||typeof data.serverUrl!=="string"||!["mpv","system"].includes(data.player)||!record(data.history)||!record(data.metadata))throw new Error("Неподдерживаемая резервная копия");
  const url=new URL(data.serverUrl);if(!["http:","https:"].includes(url.protocol)||url.username||url.password)throw new Error("Недопустимый адрес движка");
  if(Object.keys(safeMobileHistory(data.history)).length!==Object.keys(data.history).length)throw new Error("Повреждена история просмотра");
  if(Object.keys(safeMobileMetadata(data.metadata)).length!==Object.keys(data.metadata).length)throw new Error("Повреждены описания");
  for(const key of ["rutube","rutubePlaylists","rutubeQueue"])if(data[key]!==undefined&&!Array.isArray(data[key]))throw new Error("Повреждена медиатека RuTube");
  const playlists=(data.rutubePlaylists??[]).filter(item=>/^https:\/\/rutube\.ru\/plst\/\d+\/$/.test(item?.url)&&Array.isArray(item.videos)).map(item=>({url:item.url,videos:savedRutubeVideos(item.videos)}));
  return {...data,rutube:savedRutubeVideos(data.rutube??[]),rutubeQueue:savedRutubeVideos(data.rutubeQueue??[]),rutubePlaylists:playlists};
}

export function torrentFiles(raw){
  let data=raw?.data;
  if(typeof data==="string")try{data=JSON.parse(data)}catch{data={}}
  const files=raw?.file_stats??raw?.files??data?.TorrServer?.Files??[];
  return sortEpisodeFiles(files.map((file,index)=>{const path=String(file.path??file.name??file.components?.join("/")??`Файл ${index+1}`);return {id:Number(file.id??file.index??index),path,name:path.split(/[\\/]/).pop()||path,length:Number(file.length??file.size??0),...episodeNumbers(path)}}).filter(file=>VIDEO.test(file.name)));
}

export function normalizeTorrent(raw){
  const hash=String(raw?.hash??raw?.Hash??raw?.info_hash??"").toLowerCase();
  return {hash,title:String((raw?.title??raw?.name??hash)||"Без названия"),poster:String(raw?.poster??""),addedAt:Number(raw?.added_at??raw?.addedAt??raw?.timestamp??0),files:torrentFiles(raw)};
}

export function streamUrl(baseUrl,hash,file){
  return `${baseUrl.replace(/\/$/,"")}/torrents/${encodeURIComponent(hash)}/stream/${file.id}/${encodeURIComponent(file.name)}`;
}

export function mpvIntent(url,title){
  const parsed=new URL(url);const scheme=parsed.protocol.slice(0,-1);
  return `intent://${parsed.host}${parsed.pathname}${parsed.search}#Intent;scheme=${scheme};type=video/any;package=is.xyz.mpv;S.title=${encodeURIComponent(title)};S.browser_fallback_url=${encodeURIComponent(url)};end`;
}
