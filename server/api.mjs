import { createServer } from "node:http";
import { spawn } from "node:child_process";
import { MpvPlayer, sortMediaFiles } from "./mpv-player.mjs";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { basename, join } from "node:path";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import bundledFfmpegPath from "ffmpeg-static";
import { MediaDatabase } from "./database.mjs";
import { createLibrarySync } from "./library-sync.mjs";
import { createMetadataService } from "./metadata-service.mjs";
import { localizedTorrentTitle, parseTorrentTitle } from "./title-parser.mjs";
import { normalizeMagnetLink, normalizeSearchResult, normalizeTorrentFiles, TorrServerClient } from "./torrserver-client.mjs";

const apiPort=Number(process.env.TORR_LOCAL_API_PORT ?? 3001);
const projectRoot=resolve(dirname(fileURLToPath(import.meta.url)),"..");
const ffmpegPath=process.env.FFMPEG_PATH??bundledFfmpegPath;
let dataDir=process.env.TORR_DATA_DIR ?? join(process.env.APPDATA ?? process.cwd(),"TorrServerDesktop");
let posterDir=join(dataDir,"cache","posters");
try{mkdirSync(posterDir,{recursive:true})}catch{dataDir=join(process.cwd(),"local-data");posterDir=join(dataDir,"cache","posters");mkdirSync(posterDir,{recursive:true})}
const configPath=join(dataDir,"config.json");

const localConfig=readConfig();
const torrServerUrl=process.env.TORRSERVER_URL ?? localConfig.torrServerUrl ?? "http://127.0.0.1:8090";
const mpvPath=[process.env.MPV_PATH,join(projectRoot,"vendor","mpv","mpv.exe"),"C:\\Program Files\\mpv\\mpv.exe","C:\\Program Files (x86)\\mpv\\mpv.exe",join(process.env.USERPROFILE??"C:\\Users\\Public","scoop","apps","mpv","current","mpv.exe"),join(process.env.LOCALAPPDATA??"C:\\Users\\Public","Programs","mpv","mpv.exe"),"C:\\ProgramData\\chocolatey\\bin\\mpv.exe"].find(value=>value&&existsSync(value))??null;
const client=new TorrServerClient(torrServerUrl,{username:process.env.TORRSERVER_USERNAME,password:process.env.TORRSERVER_PASSWORD});
const db=new MediaDatabase(join(dataDir,"media.db"));
const {service:metadata,mode:metadataMode}=createMetadataService();
const state={online:false,syncing:false,lastSync:null,error:null,torrServerUrl,serverVersion:null,metadataMode,searchProviders:["TorrServer + Rutor"]};
const posterGeneration=new Map();
const player = new MpvPlayer({ db, client, executable: mpvPath });
const librarySync=createLibrarySync({db,client,metadata,cachePoster,state,activeHashes:()=>player.list().map(s=>s.hash)});

function readConfig(){try{return JSON.parse(readFileSync(configPath,"utf8"))}catch{return {}}}
function saveConfig(next){mkdirSync(dataDir,{recursive:true});writeFileSync(configPath,JSON.stringify(next,null,2),{encoding:"utf8",mode:0o600})}
function appSettings(){return {onboardingComplete:Boolean(localConfig.onboardingComplete),language:localConfig.language==="en"?"en":"ru",playerType:localConfig.playerType==="external"?"external":"mpv",playerPath:localConfig.playerPath??null}}

async function cachePoster(providerId,url){
  if(!providerId||!url||!/^https?:\/\//i.test(url))return null;
  const safeId=String(providerId).replace(/[^a-z0-9_-]/gi,"-");const fileName=`${safeId}.jpg`; const filePath=join(posterDir,fileName);
  if(existsSync(filePath))return fileName;
  try{const response=await fetch(url,{signal:AbortSignal.timeout(10000)});if(!response.ok)return null;const type=response.headers.get("content-type")??"";if(!type.startsWith("image/"))return null;const bytes=Buffer.from(await response.arrayBuffer());if(bytes.length>8*1024*1024)return null;writeFileSync(filePath,bytes);return fileName}catch{return null}
}

async function synchronize(full=false){await librarySync.synchronize(full);return libraryPayload()}
function libraryPayload(){const items=db.list();return {state:{...state},stats:db.stats(),items,continueWatching:items.filter((item)=>item.isStarted&&!item.isWatched).sort((a,b)=>String(b.lastPlayedAt).localeCompare(String(a.lastPlayedAt))),recentlyAdded:[...items].sort((a,b)=>String(b.addedAt).localeCompare(String(a.addedAt))).slice(0,18)}}
async function metadataServiceFind(value){return metadata.findBestMatch(value)}

function sendJson(res,status,payload,origin){res.writeHead(status,{"content-type":"application/json; charset=utf-8","access-control-allow-origin":origin,"cache-control":"no-store"});res.end(JSON.stringify(payload))}
async function readJson(req){const chunks=[];for await(const chunk of req)chunks.push(chunk);return chunks.length?JSON.parse(Buffer.concat(chunks).toString("utf8")):{} }
function allowedOrigin(req){const origin=req.headers.origin??"http://localhost:3000";return /^http:\/\/(?:localhost|127\.0\.0\.1)(?::\d+)?$/.test(origin)?origin:"http://localhost:3000"}
function transcodeUrl(hash,fileIndex,fileName,videoCodec="h264"){const video=/^(?:h264|avc1)$/i.test(videoCodec??"")?"copy":"h264";return `http://127.0.0.1:${apiPort}/api/transcode/${encodeURIComponent(hash)}?index=${fileIndex}&name=${encodeURIComponent(fileName||"video")}&video=${video}`}
function probeMediaOnce(hash,fileIndex,fileName){return new Promise(resolve=>{if(!ffmpegPath||!existsSync(ffmpegPath))return resolve({durationSeconds:null,audioCodec:null,videoCodec:null});const source=client.streamUrl(hash,fileIndex,fileName);const process=spawn(ffmpegPath,["-hide_banner","-i",source],{windowsHide:true,stdio:["ignore","ignore","pipe"]});let output="";const timer=setTimeout(()=>process.kill("SIGTERM"),15000);process.stderr.on("data",chunk=>{output=(output+chunk.toString()).slice(-30000)});process.on("close",()=>{clearTimeout(timer);const match=output.match(/Duration:\s*(\d+):(\d+):(\d+(?:\.\d+)?)/);const durationSeconds=match?Number(match[1])*3600+Number(match[2])*60+Number(match[3]):null;const audioCodec=output.match(/Audio:\s*([^,\s]+)/)?.[1]??null;const videoCodec=output.match(/Video:\s*([^,\s]+)/)?.[1]??null;resolve({durationSeconds,audioCodec,videoCodec})});process.on("error",()=>{clearTimeout(timer);resolve({durationSeconds:null,audioCodec:null,videoCodec:null})})})}
async function probeMedia(hash,fileIndex,fileName){let result={durationSeconds:null,audioCodec:null,videoCodec:null};for(let attempt=0;attempt<3;attempt++){result=await probeMediaOnce(hash,fileIndex,fileName);if(result.durationSeconds&&result.videoCodec)return result;await new Promise(resolve=>setTimeout(resolve,700*(attempt+1)))}return result}
function generateFramePoster(hash){
  const fileName=`frame-${hash}.jpg`;const filePath=join(posterDir,fileName);if(existsSync(filePath))return Promise.resolve(fileName);if(posterGeneration.has(hash))return posterGeneration.get(hash);
  const promise=(async()=>{if(!ffmpegPath||!existsSync(ffmpegPath))throw new Error("FFmpeg не установлен");const details=await client.torrentStats(hash);const files=normalizeTorrentFiles(details);const main=[...files].sort((a,b)=>b.length-a.length)[0];if(!main)throw new Error("Видео не найдено");const source=client.streamUrl(hash,main.id,main.name);await new Promise((resolve,reject)=>{const process=spawn(ffmpegPath,["-hide_banner","-loglevel","error","-ss","30","-i",source,"-map","0:v:0","-frames:v","1","-vf","scale=600:-2","-q:v","3","-y",filePath],{windowsHide:true,stdio:["ignore","ignore","pipe"]});let error="";const timer=setTimeout(()=>{process.kill("SIGTERM");reject(new Error("Создание постера превысило 30 секунд"))},30000);process.stderr.on("data",chunk=>{error=(error+chunk.toString()).slice(-2000)});process.on("error",reject);process.on("close",code=>{clearTimeout(timer);if(code===0&&existsSync(filePath))resolve();else reject(new Error(error||`FFmpeg ${code}`))})});return fileName})().finally(()=>posterGeneration.delete(hash));posterGeneration.set(hash,promise);return promise;
}
function posterPlaceholder(title,hash){const words=String(title||"Без названия").split(/\s+/);const lines=[];let line="";for(const word of words){if(`${line} ${word}`.trim().length>20&&line){lines.push(line);line=word}else line=`${line} ${word}`.trim()}if(line)lines.push(line);const hue=parseInt(hash.slice(0,6),16)%360;const text=lines.slice(0,5).map((value,index)=>`<text x="300" y="${490+index*58}" text-anchor="middle" fill="#eef7ff" font-family="Segoe UI,Arial" font-size="${index?34:40}" font-weight="600">${escapeXml(value)}</text>`).join("");return Buffer.from(`<svg xmlns="http://www.w3.org/2000/svg" width="600" height="900"><defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop stop-color="hsl(${hue} 55% 24%)"/><stop offset="1" stop-color="#07111d"/></linearGradient></defs><rect width="600" height="900" fill="url(#g)"/><circle cx="300" cy="280" r="88" fill="none" stroke="#28a7ff" stroke-width="8" opacity=".9"/><path d="M278 235l72 45-72 45z" fill="#28a7ff"/><text x="300" y="800" text-anchor="middle" fill="#6f8ba3" font-family="Segoe UI,Arial" font-size="24" letter-spacing="4">PIRATE CINEMA</text>${text}</svg>`)}
function escapeXml(value){return String(value).replace(/[&<>"']/g,char=>({"&":"&amp;","<":"&lt;",">":"&gt;",'"':"&quot;","'":"&apos;"})[char])}
function streamWithCompatibleAudio(req,res,{hash,fileIndex,fileName,origin,videoMode,startSeconds=0}){
  if(!ffmpegPath||!existsSync(ffmpegPath))return sendJson(res,503,{error:"FFmpeg не установлен"},origin);
  const source=client.streamUrl(hash,fileIndex,fileName);const videoArgs=videoMode==="h264"?["-c:v","libx264","-preset","veryfast","-crf","21","-pix_fmt","yuv420p"]:["-c:v","copy"];const seekArgs=startSeconds>0?["-ss",String(startSeconds)]:[];const args=["-hide_banner","-loglevel","error",...seekArgs,"-i",source,"-map","0:v:0","-map","0:a:0?",...videoArgs,"-c:a","aac","-b:a","192k","-ac","2","-movflags","frag_keyframe+empty_moov+default_base_moof","-f","mp4","pipe:1"];
  const process=spawn(ffmpegPath,args,{windowsHide:true,stdio:["ignore","pipe","pipe"]});let headersSent=false;let stderr="";
  process.stdout.once("data",chunk=>{headersSent=true;res.writeHead(200,{"content-type":"video/mp4","access-control-allow-origin":origin,"cache-control":"no-store","accept-ranges":"none","x-stream-start":String(startSeconds)});res.write(chunk);process.stdout.pipe(res)});
  process.stderr.on("data",chunk=>{stderr=(stderr+chunk.toString()).slice(-2000)});
  process.on("error",error=>{if(!headersSent&&!res.headersSent)sendJson(res,500,{error:`FFmpeg: ${error.message}`},origin)});
  process.on("close",code=>{if(!headersSent&&!res.headersSent)sendJson(res,502,{error:stderr.trim()||`FFmpeg завершился с кодом ${code}`},origin);else if(!res.writableEnded)res.end()});
  req.on("close",()=>{if(!process.killed)process.kill("SIGTERM")});
}

async function torrentFiles(hash) {
  if (!state.online) throw new Error("TorrServer недоступен");
  const saved = (await client.listTorrents()).find(item => String(item.hash ?? item.Hash ?? "").toLowerCase() === hash);
  if (!saved) throw new Error("Раздача больше не сохранена в TorrServer");
  const cached = normalizeTorrentFiles(saved);
  const details = cached.length ? saved : await client.torrentStats(hash);
  const files = sortMediaFiles(cached.length ? cached : normalizeTorrentFiles(details));
  if (!files.length) throw new Error("TorrServer ещё получает список видеофайлов. Повторите позже.");
  return { title: details?.title ?? details?.name ?? hash, files };
}

const server=createServer(async(req,res)=>{
  const origin=allowedOrigin(req); const url=new URL(req.url??"/",`http://127.0.0.1:${apiPort}`);
  if(req.method==="OPTIONS"){res.writeHead(204,{"access-control-allow-origin":origin,"access-control-allow-methods":"GET,POST,DELETE,OPTIONS","access-control-allow-headers":"content-type"});return res.end()}
  try{
    if(req.method==="GET"&&url.pathname==="/api/health")return sendJson(res,200,{ok:true,...state,database:join(dataDir,"media.db")},origin);
    if(req.method==="POST"&&url.pathname==="/api/maintenance/checkpoint"){db.checkpoint();return sendJson(res,200,{saved:true},origin)}
    if(req.method==="GET"&&url.pathname==="/api/diagnostics"){
      const settings=appSettings();const checks=[{id:"api",ok:true,detail:`127.0.0.1:${apiPort}`},{id:"torrserver",ok:state.online,detail:state.online?state.serverVersion:"TorrServer не отвечает"},{id:"mpv",ok:Boolean(mpvPath&&existsSync(mpvPath)),detail:mpvPath??"MPV не найден"},{id:"ffmpeg",ok:Boolean(ffmpegPath&&existsSync(ffmpegPath)),detail:ffmpegPath??"FFmpeg не найден"},{id:"player",ok:settings.playerType==="mpv"||Boolean(settings.playerPath&&existsSync(settings.playerPath)),detail:settings.playerType==="mpv"?"Встроенный MPV":settings.playerPath??"Плеер не выбран"},{id:"database",ok:existsSync(join(dataDir,"media.db")),detail:join(dataDir,"media.db")}];return sendJson(res,200,{ok:checks.every(item=>item.ok),checks},origin)
    }
    if(req.method==="GET"&&url.pathname==="/api/library")return sendJson(res,200,libraryPayload(),origin);
    if(req.method==="POST"&&url.pathname==="/api/sync"){
      const body=await readJson(req);
      return sendJson(res,200,await synchronize(body.full!==false),origin);
    }
    const detailMatch=url.pathname.match(/^\/api\/torrents\/([a-f0-9]{40})\/details$/i);
    if(req.method==="GET"&&detailMatch){
      const hash=detailMatch[1].toLowerCase();
      if(!db.get(hash))return sendJson(res,404,{error:"Фильм ещё не сохранён в медиатеке"},origin);
      let warning=null;
      try{await librarySync.enrich(hash)}catch{warning="Сервис описаний временно недоступен"}
      return sendJson(res,200,{item:librarySync.item(hash),warning},origin);
    }
    if(req.method==="GET"&&url.pathname==="/api/settings")return sendJson(res,200,{torrServerUrl:state.torrServerUrl,metadataMode,metadataConfigured:metadataMode==="tmdb",dataDir,...appSettings()},origin);
    if(req.method==="POST"&&url.pathname==="/api/settings"){
      const body=await readJson(req);const next={...localConfig};let restartRequired=false;
      if(body.torrServerUrl!==undefined){const nextUrl=new URL(body.torrServerUrl);if(!["http:","https:"].includes(nextUrl.protocol))throw new Error("Only HTTP(S) TorrServer addresses are allowed");next.torrServerUrl=nextUrl.toString().replace(/\/$/,"");restartRequired=next.torrServerUrl!==localConfig.torrServerUrl}
      if(body.language!==undefined){if(!["ru","en"].includes(body.language))return sendJson(res,400,{error:"Unsupported language"},origin);next.language=body.language}
      if(body.playerType!==undefined){if(!["mpv","external"].includes(body.playerType))return sendJson(res,400,{error:"Unsupported player"},origin);next.playerType=body.playerType}
      if(body.playerPath!==undefined){const path=String(body.playerPath??"").trim();if(path&&(!/\.exe$/i.test(path)||!existsSync(path)))return sendJson(res,400,{error:"Выбранный EXE-файл плеера не найден"},origin);next.playerPath=path||null}
      if(next.playerType==="external"&&!next.playerPath)return sendJson(res,400,{error:"Сначала выберите EXE-файл локального плеера"},origin);
      if(body.onboardingComplete!==undefined)next.onboardingComplete=Boolean(body.onboardingComplete);
      Object.assign(localConfig,next);saveConfig(localConfig);return sendJson(res,200,{saved:true,restartRequired,...appSettings()},origin);
    }
    const framePosterMatch=url.pathname.match(/^\/api\/posters\/frame\/([a-f0-9]{40})$/i);
    if(req.method==="GET"&&framePosterMatch){const hash=framePosterMatch[1].toLowerCase();try{const name=await generateFramePoster(hash);const path=join(posterDir,name);res.writeHead(200,{"content-type":"image/jpeg","cache-control":"public, max-age=31536000, immutable","access-control-allow-origin":origin});return res.end(readFileSync(path))}catch{const svg=posterPlaceholder(db.get(hash)?.title??db.get(hash)?.torrent_name??"Без названия",hash);res.writeHead(200,{"content-type":"image/svg+xml; charset=utf-8","cache-control":"public, max-age=3600","access-control-allow-origin":origin});return res.end(svg)}}
    if(req.method==="GET"&&url.pathname.startsWith("/api/posters/")){
      const name=basename(decodeURIComponent(url.pathname.slice("/api/posters/".length)));if(!/^[a-z0-9_-]+\.jpg$/i.test(name))return sendJson(res,400,{error:"Invalid poster"},origin);
      const path=join(posterDir,name);if(!existsSync(path))return sendJson(res,404,{error:"Poster not found"},origin);res.writeHead(200,{"content-type":"image/jpeg","cache-control":"public, max-age=31536000, immutable","access-control-allow-origin":origin});return res.end(readFileSync(path));
    }
    const removeMatch=url.pathname.match(/^\/api\/torrents\/([a-f0-9]{40})$/i);
    if(req.method==="DELETE"&&removeMatch){
      const hash=removeMatch[1].toLowerCase();if(player.list().some(s=>s.hash===hash))return sendJson(res,409,{error:"Сначала закройте MPV с этой раздачей"},origin);if(!state.online)await synchronize();if(!state.online)return sendJson(res,503,{error:"TorrServer не запущен"},origin);
      await client.removeTorrent(hash);db.remove(hash);return sendJson(res,200,{removed:true,hash},origin);
    }
    const filesMatch=url.pathname.match(/^\/api\/torrents\/([a-f0-9]{40})\/files$/i);
    if(req.method==="GET"&&filesMatch){
      if(!state.online)return sendJson(res,409,{error:"TorrServer недоступен"},origin);
      const hash=filesMatch[1].toLowerCase();const details=await torrentFiles(hash);
      const history=new Map(db.listFileHistory(hash).map(entry=>[Number(entry.fileIndex),entry]));
      const files=details.files.map(file=>{const played=history.get(file.id);const duration=Number(played?.playbackDuration)||0,timecode=Number(played?.playbackTimecode)||0;return {...file,viewed:Boolean(played?.isWatched),resumeSeconds:timecode,durationSeconds:duration,progress:duration?Math.max(0,Math.min(100,Math.round(timecode/duration*100))):null,lastPlayedAt:played?.lastPlayedAt??null,launchCount:Number(played?.launchCount??0)}});
      return sendJson(res,200,{hash,title:details.title,files},origin);
    }
    const transcodeMatch=url.pathname.match(/^\/api\/transcode\/([^/]+)$/);
    if(req.method==="GET"&&transcodeMatch){
      if(!state.online)return sendJson(res,409,{error:"TorrServer недоступен"},origin);const hash=decodeURIComponent(transcodeMatch[1]);const fileIndex=Math.max(1,Number(url.searchParams.get("index"))||1);const fileName=url.searchParams.get("name")||"video";const videoMode=url.searchParams.get("video")==="h264"?"h264":"copy";const requestedStart=Number(url.searchParams.get("start")??0);const startSeconds=Number.isFinite(requestedStart)?Math.max(0,requestedStart):0;
      return streamWithCompatibleAudio(req,res,{hash,fileIndex,fileName,origin,videoMode,startSeconds});
    }
    if(req.method==="GET"&&url.pathname==="/api/mpv/sessions")return sendJson(res,200,{sessions:player.list()},origin);
    const controlMatch=url.pathname.match(/^\/api\/mpv\/sessions\/([0-9-]+)$/);
    if(req.method==="POST"&&controlMatch){
      const body=await readJson(req);const sessions=await player.control(controlMatch[1],body.action,body.value);
      return sendJson(res,200,{sessions},origin);
    }
    const historyMatch=url.pathname.match(/^\/api\/torrents\/([a-f0-9]{40})\/files\/([0-9]+)\/history$/i);
    const playbackDiagnosticMatch=url.pathname.match(/^\/api\/torrents\/([a-f0-9]{40})\/files\/([0-9]+)\/diagnostics$/i);
    if(req.method==="GET"&&playbackDiagnosticMatch){
      const hash=playbackDiagnosticMatch[1].toLowerCase(),index=Number(playbackDiagnosticMatch[2]);let files=[];let torrentError=null;try{files=(await torrentFiles(hash)).files}catch(error){torrentError=error instanceof Error?error.message:String(error)}const file=files.find(item=>item.id===index);const settings=appSettings();const checks=[{id:"torrserver",ok:state.online,detail:state.online?state.serverVersion:"Нет соединения"},{id:"torrent",ok:!torrentError,detail:torrentError??"Раздача доступна"},{id:"file",ok:Boolean(file),detail:file?`${file.name} · ${file.length} bytes`:"Файл не найден в раздаче"},{id:"player",ok:settings.playerType==="mpv"?Boolean(mpvPath&&existsSync(mpvPath)):Boolean(settings.playerPath&&existsSync(settings.playerPath)),detail:settings.playerType==="mpv"?(mpvPath??"MPV не найден"):(settings.playerPath??"Плеер не выбран")}];return sendJson(res,200,{ok:checks.every(item=>item.ok),checks,recommendation:!state.online?"Перезапустите TorrServer":torrentError?"Обновите раздачу и повторите":!file?"Выберите другой файл":"Основные компоненты готовы; проверьте наличие пиров и повторите запуск"},origin)
    }
    if(req.method==="POST"&&historyMatch){
      const hash=historyMatch[1].toLowerCase(),index=Number(historyMatch[2]);const body=await readJson(req);
      if(!Number.isSafeInteger(index)||index<1)return sendJson(res,400,{error:"Некорректный индекс файла"},origin);
      if(body.action==="viewed"&&typeof body.value==="boolean")db.setFileViewed(hash,index,body.value);
      else if(body.action==="resetPosition"){
        if(player.isOpen(hash,index))return sendJson(res,409,{error:"Закройте окно этого файла в MPV перед сбросом позиции"},origin);
        db.resetFileProgress(hash,index);
      }else return sendJson(res,400,{error:"Неизвестное действие истории"},origin);
      return sendJson(res,200,{saved:true},origin);
    }
    const mpvMatch=url.pathname.match(/^\/api\/mpv\/([a-f0-9]{40})$/i);
    if(req.method==="POST"&&mpvMatch){
      if(!state.online)return sendJson(res,409,{error:"TorrServer недоступен"},origin);
      const hash=mpvMatch[1].toLowerCase();const body=await readJson(req);const fileIndex=Number(body.fileIndex);
      if(!Number.isSafeInteger(fileIndex)||fileIndex<1)return sendJson(res,400,{error:"Некорректный индекс файла"},origin);
      const {files}=await torrentFiles(hash);
      if(appSettings().playerType==="external"){
        const executable=appSettings().playerPath;if(!executable||!existsSync(executable))return sendJson(res,503,{error:"Выбранный локальный плеер не найден. Измените его в настройках."},origin);
        const file=files.find(item=>item.id===fileIndex);if(!file)return sendJson(res,404,{error:"Видеофайл не найден"},origin);
        db.markFilePlayed(hash,{fileIndex,fileName:file.name,filePath:file.path});const child=spawn(executable,[client.streamUrl(hash,file.id,file.name)],{windowsHide:false,stdio:"ignore",detached:true});child.unref();
        return sendJson(res,200,{launched:true,player:"external",playerName:basename(executable)},origin);
      }
      if(!mpvPath)return sendJson(res,503,{error:"Встроенный MPV не найден. Проверьте файлы приложения или задайте MPV_PATH"},origin);
      const session=await player.play({hash,files,fileIndex,mode:body.mode,existing:body.existing,sessionId:body.sessionId,autoNext:body.autoNext});
      return sendJson(res,200,{launched:true,player:"mpv",...session},origin);
    }
    if(req.method==="POST"&&url.pathname.startsWith("/api/play/")){
      if(!state.online)return sendJson(res,409,{error:"TorrServer is offline"},origin);const hash=decodeURIComponent(url.pathname.slice("/api/play/".length)).toLowerCase();const body=await readJson(req);const fileIndex=Number(body.fileIndex)||1;const fileName=body.fileName||"video";const filePath=String(body.filePath||"").slice(0,1000)||null;const media=await probeMedia(hash,fileIndex,fileName);if(!media.videoCodec)return sendJson(res,425,{error:"TorrServer ещё не получил видеоданные этой раздачи. Проверьте наличие пиров и повторите запуск через несколько секунд."},origin);db.markFilePlayed(hash,{fileIndex,fileName,filePath});return sendJson(res,200,{url:client.streamUrl(hash,fileIndex,fileName),transcodedUrl:transcodeUrl(hash,fileIndex,fileName,media.videoCodec),viewed:true,...media},origin);
    }
    if(req.method==="GET"&&url.pathname==="/api/search"){
      const query=(url.searchParams.get("q")??"").trim();if(query.length<2)return sendJson(res,400,{error:"Введите минимум 2 символа"},origin);
      const [torrServerSettled,details]=await Promise.all([client.searchTorrents(query).then(value=>({online:true,value})).catch(()=>({online:false,value:[]})),metadataServiceFind(query).catch(()=>null)]);const torrServerItems=torrServerSettled.value.map(raw=>({...normalizeSearchResult(raw),source:"TorrServer"})).filter(item=>item.magnet);
      return sendJson(res,200,{query,metadata:details,results:torrServerItems,providers:[{name:"TorrServer",online:torrServerSettled.online,count:torrServerItems.length}],cached:false},origin);
    }
    if(req.method==="POST"&&url.pathname==="/api/torrents/add"){
      if(!state.online)return sendJson(res,409,{error:"TorrServer недоступен"},origin);const body=await readJson(req);
      const normalized=normalizeMagnetLink(body.magnet);const title=String(body.title||normalized.title||"Magnet-раздача").trim();
      const added=await client.addTorrent({magnet:normalized.magnet,title,poster:body.poster??null,category:body.category??""});
      if(!db.get(added.hash)){
        const parsed=parseTorrentTitle(title);
        db.upsert({torrentHash:added.hash,torrentName:title,title:localizedTorrentTitle(title)??parsed.title,year:parsed.year,posterUrl:body.poster??null});
      }
      return sendJson(res,200,{added:true,alreadyExists:added.alreadyExists,hash:added.hash,title,item:librarySync.item(added.hash)},origin);
    }
    return sendJson(res,404,{error:"Not found"},origin);
  }catch(error){const aborted=error?.name==="AbortError";const rejected=/^TorrServer (?:400|409|500)$/.test(error?.message??"");const message=aborted||rejected?"TorrServer ещё получает данные раздачи. Проверьте наличие пиров и повторите через несколько секунд.":error.message;return sendJson(res,error.status??(aborted?504:rejected?425:400),{error:message,active:error.active},origin)}
});

server.listen(apiPort,"127.0.0.1",()=>{console.log(`[local-api] http://127.0.0.1:${server.address().port} · SQLite ${join(dataDir,"media.db")} · metadata ${metadataMode}`);synchronize()});
const interval=setInterval(()=>synchronize(),60000); interval.unref();
for(const signal of ["SIGINT","SIGTERM"]){process.on(signal,()=>{clearInterval(interval);player.close();db.close();server.close(()=>process.exit(0))})}
