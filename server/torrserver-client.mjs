export class TorrServerClient {
  constructor(baseUrl, {username,password}={}) { this.baseUrl=baseUrl.replace(/\/$/,""); this.auth=username ? `Basic ${Buffer.from(`${username}:${password ?? ""}`).toString("base64")}` : null; }
  async request(path, init={}) {
    const {timeoutMs=6000,...fetchInit}=init;const controller=new AbortController(); const timer=setTimeout(()=>controller.abort(),timeoutMs);
    try {
      const headers={accept:"application/json",...(fetchInit.body?{"content-type":"application/json"}:{}),...(this.auth?{authorization:this.auth}:{})};
      const response=await fetch(`${this.baseUrl}${path}`,{...fetchInit,headers:{...headers,...fetchInit.headers},signal:controller.signal});
      if(!response.ok) throw new Error(`TorrServer ${response.status}`);
      const text=await response.text();
      try{return JSON.parse(text)}catch{return text}
    } finally { clearTimeout(timer); }
  }
  async echo(){const result=await this.request("/echo");return String(result).trim()}
  async listTorrents(){const result=await this.request("/torrents",{method:"POST",body:JSON.stringify({action:"list"})});const items=Array.isArray(result)?result:(result?.torrents??result?.items);if(!Array.isArray(items))throw new Error("Некорректный список раздач TorrServer");return items}
  async listViewed(){
    try{const result=await this.request("/viewed",{method:"POST",body:JSON.stringify({action:"list"})});return result??[]}
    catch(error){
      // Some MatriX builds return 500 until the first playback entry exists.
      // Viewed history is optional and must not make a healthy TorrServer offline.
      if(error instanceof Error&&/TorrServer (?:404|500)/.test(error.message))return [];
      throw error;
    }
  }
  async searchTorrents(query){const result=await this.request(`/search?query=${encodeURIComponent(query)}`);return Array.isArray(result)?result:[]}
  async torrentStats(hash){let lastError;for(let attempt=0;attempt<3;attempt++){try{return await this.request(`/stream/details?link=${encodeURIComponent(hash)}&stat`,{timeoutMs:20000})}catch(error){lastError=error;const retryable=error?.name==="AbortError"||(error instanceof Error&&/^TorrServer (?:400|409|500)$/.test(error.message));if(!retryable||attempt===2)throw error;await new Promise(resolve=>setTimeout(resolve,1000*(attempt+1)))}}throw lastError}
  async addTorrent({magnet,title,poster=null,category=""}){const normalized=normalizeMagnetLink(magnet);const existing=(await this.listTorrents()).find(item=>String(item.hash??item.Hash??item.info_hash??"").toLowerCase()===normalized.hash);if(existing)return {result:existing,hash:normalized.hash,magnet:normalized.magnet,alreadyExists:true};const result=await this.request("/torrents",{method:"POST",body:JSON.stringify({action:"add",link:normalized.magnet,title:String(title??"").trim(),poster:poster||undefined,category:String(category??""),save_to_db:true})});const responseHash=normalizeInfoHash(result?.hash??result?.Hash??result?.info_hash??result?.torrent_hash);return {result,hash:responseHash??normalized.hash,magnet:normalized.magnet,alreadyExists:false}}
  async removeTorrent(hash){return this.request("/torrents",{method:"POST",body:JSON.stringify({action:"rem",hash})})}
  streamUrl(hash,fileIndex=1,fileName="video"){return `${this.baseUrl}/stream/${encodeURIComponent(fileName)}?link=${encodeURIComponent(hash)}&index=${fileIndex}&play`}
}

export function normalizeTorrentFiles(raw){
  const files=Array.isArray(raw?.file_stats)?raw.file_stats:Array.isArray(raw?.files)?raw.files:[];
  return files.map((file,index)=>{
    const path=String(file.path??file.name??`Файл ${index+1}`);const seasonMatch=path.match(/(?:^|[\\/ ._-])(?:S(?:eason[ ._-]?)?|Сезон[ ._-]*)(\d{1,2})(?:[\\/ ._-]|E\d|$)/i);const episodeMatch=path.match(/E(?:pisode[ ._-]?)?(\d{1,3})(?:[^0-9]|$)/i)??path.split(/[\\/]/).pop()?.match(/^(\d{1,3})(?:[ ._-]|$)/);
    return {id:Number(file.id??file.index??index+1),path,name:path.split(/[\\/]/).pop()||path,length:Number(file.length??file.size??0),season:seasonMatch?Number(seasonMatch[1]):null,episode:episodeMatch?Number(episodeMatch[1]):null};
  }).filter(file=>/\.(?:mkv|mp4|avi|mov|m4v|webm|ts|m2ts)$/i.test(file.name));
}

export function normalizeSearchResult(raw){const rawHash=normalizeInfoHash(raw.Hash??raw.hash??raw.info_hash);const magnet=safeMagnet(raw.Magnet??raw.magnet??"")||(rawHash?`magnet:?xt=urn:btih:${rawHash}`:"");return {title:String(raw.Title??raw.title??raw.Name??raw.name??"Без названия"),size:String(raw.Size??raw.size??"—"),seeders:Number(raw.Seed??raw.seed??raw.seeders??0),peers:Number(raw.Peer??raw.peer??raw.peers??0),magnet,hash:extractInfoHash(magnet)??rawHash??"",tracker:String(raw.Tracker??raw.tracker??""),year:Number(raw.Year??raw.year)||null,imdbId:String(raw.IMDBID??raw.imdbid??"")||null,quality:qualityLabel(raw),createdAt:String(raw.CreateDate??raw.createDate??"")};}
function qualityLabel(raw){const title=String(raw.Title??raw.title??"");const match=title.match(/(?:2160p|4K|1080p|720p|480p)/i);if(match)return match[0].toUpperCase()==="4K"?"2160p":match[0].toLowerCase();const value=Number(raw.VideoQuality??raw.videoQuality??0);if(value>=300)return "2160p";if(value>=200)return "1080p";if(value>=100)return "720p";return "Другое"}

export function normalizeTorrent(raw) {
  const stat=raw.stat ?? raw.status ?? {};
  const hash=raw.hash ?? raw.Hash ?? raw.info_hash ?? raw.id;
  return {hash:String(hash ?? ""),name:raw.title ?? raw.name ?? raw.TorrentName ?? String(hash ?? "Unknown title"),poster:raw.poster ?? null,
    addedAt:raw.added_at ?? raw.timestamp ?? null,status:String(stat.stat_string ?? stat.status ?? raw.status ?? "saved"),
    downloadSpeed:Number(stat.download_speed ?? stat.downloadSpeed ?? raw.download_speed ?? 0),uploadSpeed:Number(stat.upload_speed ?? stat.uploadSpeed ?? raw.upload_speed ?? 0),
    peers:Number(stat.active_peers ?? stat.peers ?? raw.peers ?? 0),isActive:Boolean(stat.active_peers || stat.download_speed || raw.active)};
}

export function normalizeViewed(raw) {
  const values=Array.isArray(raw)?raw:Object.entries(raw ?? {}).map(([hash,value])=>({hash,...(typeof value==="object"?value:{timecode:value})}));
  return values.map((item)=>({hash:String(item.hash ?? item.Hash ?? item.torrent_hash ?? item.id ?? ""),timecode:Number(item.timecode ?? item.time ?? item.position ?? 0),duration:Number(item.duration ?? item.total ?? item.length ?? 0),lastPlayedAt:item.last_played_at ?? item.updated_at ?? new Date().toISOString()})).filter((item)=>item.hash);
}

export function normalizeMagnetLink(input){
  let value=String(input??"").trim().replace(/^['"]+|['"]+$/g,"").replace(/&amp;/gi,"&");
  const offset=value.toLowerCase().indexOf("magnet:?");if(offset>0)value=value.slice(offset);
  if(!/^magnet:\?/i.test(value))throw new Error("Нужна magnet-ссылка, начинающаяся с magnet:?");
  let parsed;try{parsed=new URL(value)}catch{throw new Error("Magnet-ссылка повреждена")}
  const xt=parsed.searchParams.getAll("xt").find(item=>/^urn:btih:/i.test(item));const hash=normalizeInfoHash(xt?.slice("urn:btih:".length));
  if(!hash)throw new Error("Magnet-ссылка не содержит корректный BTIH hash")
  const params=new URLSearchParams();for(const [key,current] of parsed.searchParams){if(key.toLowerCase()==="xt"&&/^urn:btih:/i.test(current))continue;params.append(key,current)}params.append("xt",`urn:btih:${hash}`);
  return {magnet:`magnet:?${params.toString()}`,hash,title:(parsed.searchParams.get("dn")??"").trim()};
}

export function extractInfoHash(input){try{return normalizeMagnetLink(input).hash}catch{return normalizeInfoHash(input)}}

function safeMagnet(input){try{return normalizeMagnetLink(input).magnet}catch{return ""}}
function normalizeInfoHash(input){const value=String(input??"").trim();if(/^[a-f0-9]{40}$/i.test(value))return value.toLowerCase();if(/^[a-z2-7]{32}$/i.test(value))return base32ToHex(value);return null}
function base32ToHex(input){const alphabet="ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";let bits=0,value=0;const bytes=[];for(const character of input.toUpperCase()){const index=alphabet.indexOf(character);if(index<0)return null;value=(value<<5)|index;bits+=5;if(bits>=8){bytes.push((value>>>(bits-8))&255);bits-=8}}return bytes.length===20?bytes.map(byte=>byte.toString(16).padStart(2,"0")).join(""):null}
