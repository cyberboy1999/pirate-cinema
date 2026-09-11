"use client";

import {useEffect,useRef,useState} from "react";
import {Check,Play,X} from "@phosphor-icons/react";

const API="http://127.0.0.1:3001";
export type TorrentFile={id:number;path:string;name:string;length:number;season:number|null;episode:number|null;viewed:boolean;lastPlayedAt:string|null;launchCount:number;resumeSeconds:number;durationSeconds:number;progress:number|null};
type Session={id:string;hash:string;fileIndex:number;fileName:string;timecode:number;duration:number;loading:boolean;ended:boolean;autoNext:boolean;error:string|null;canNext:boolean;nextName:string|null};
type PlayChoice={file:TorrentFile;mode:"resume"|"start";active:Session[]};
class RequestError extends Error {
  constructor(message:string,public active:Session[]=[]){super(message)}
}
async function request(path:string,body?:object){
  const response=await fetch(API+path,{method:body?"POST":"GET",headers:body?{"content-type":"application/json"}:undefined,body:body?JSON.stringify(body):undefined,cache:"no-store"});
  const payload=await response.json();
  if(!response.ok)throw new RequestError(payload.error||"Локальный API недоступен",payload.active??[]);
  return payload;
}
function clock(value:number){const total=Math.max(0,Math.floor(value||0));return [Math.floor(total/3600),Math.floor(total%3600/60),total%60].map((v,i)=>i?String(v).padStart(2,"0"):String(v)).join(":")}
function group(file:TorrentFile){return file.path.split(/[\\/]/).slice(0,-1).join(" / ")||"Видео"}
function size(value:number){return value>=1024**3?(value/1024**3).toFixed(1)+" ГБ":Math.round(value/1024**2)+" МБ"}

export function MediaFilePicker({item,files,busy,error,onClose,onRetry,onRefresh,onLaunched}:{
  item:{torrentHash:string;title:string;year?:number|null;posterUrl?:string|null;posterPath?:string|null;genres?:string[];rating?:number|null;overview?:string;overviewSourceUrl?:string|null;runtimeSeconds?:number|null};files:TorrentFile[];busy:boolean;error:string;
  onClose:()=>void;onRetry:()=>void;onRefresh:()=>Promise<void>;onLaunched:(message:string)=>void;
}){
  const heading=useRef<HTMLHeadingElement>(null);
  const [details,setDetails]=useState(item);
  const [descriptionBusy,setDescriptionBusy]=useState(true);
  const [descriptionError,setDescriptionError]=useState("");
  const [choice,setChoice]=useState<PlayChoice|null>(null);
  const [working,setWorking]=useState(false);
  const [failure,setFailure]=useState("");
  const [autoNext,setAutoNext]=useState(false);
  useEffect(()=>{
    heading.current?.focus();
    const controller=new AbortController();
    void fetch(API+"/api/torrents/"+item.torrentHash+"/details",{signal:controller.signal,cache:"no-store"})
      .then(async response=>{const data=await response.json();if(!response.ok)throw new Error(data.error);return data})
      .then(data=>{if(!controller.signal.aborted){if(data.item)setDetails(data.item);setDescriptionError(data.warning??"")}})
      .catch(error=>{if(!controller.signal.aborted)setDescriptionError(error.message||"Описание временно недоступно")})
      .finally(()=>{if(!controller.signal.aborted)setDescriptionBusy(false)});
    return()=>controller.abort();
  },[item.torrentHash]);
  const groups=files.reduce<Record<string,TorrentFile[]>>((result,file)=>{(result[group(file)]??=[]).push(file);return result},{});
  async function play(file:TorrentFile,mode:"resume"|"start",existing="ask",sessionId?:string){
    if(working)return;
    setWorking(true);setFailure("");
    try{
      const payload=await request("/api/mpv/"+item.torrentHash,{fileIndex:file.id,mode,existing,sessionId,autoNext});
      onLaunched(payload.focused?"Этот файл уже открыт — переключаюсь на MPV":"Файл передан в MPV");
      onClose();
    }catch(error){
      if(error instanceof RequestError&&error.active.length)setChoice({file,mode,active:error.active});
      else setFailure(error instanceof Error?error.message:"Не удалось открыть файл");
    }finally{setWorking(false)}
  }
  async function updateHistory(file:TorrentFile,action:"viewed"|"resetPosition"){
    if(working)return;
    if(action==="resetPosition"&&!window.confirm("Сбросить позицию для «"+file.name+"»? Отметка «Просмотрено» сохранится."))return;
    setWorking(true);setFailure("");
    try{await request("/api/torrents/"+item.torrentHash+"/files/"+file.id+"/history",{action,value:!file.viewed});await onRefresh()}
    catch(error){setFailure(error instanceof Error?error.message:"Не удалось сохранить историю")}
    finally{setWorking(false)}
  }
  const poster=details.posterPath?API+details.posterPath:details.posterUrl;
  return <section className="file-dialog media-detail-page" aria-labelledby="file-picker-title">
    <header><div><span>О ФИЛЬМЕ</span><h2 ref={heading} tabIndex={-1} id="file-picker-title">{details.title}</h2><p>{[details.year,details.runtimeSeconds?Math.round(details.runtimeSeconds/60)+" мин":null,details.rating?"★ "+details.rating:null].filter(Boolean).join(" · ")}</p></div><button disabled={working} onClick={onClose} aria-label="Вернуться назад"><X size={22}/></button></header>
    <div className="media-description">
      {poster&&<img src={poster} alt={"Постер: "+details.title} onError={event=>{event.currentTarget.hidden=true}}/>}
      <div><p className="description-genres">{details.genres?.join(" · ")}</p><h3>Описание</h3>
        <p className="description-text">{details.overview||(descriptionBusy?"Загружаем описание…":"Описание пока не найдено. Вы можете выбрать файл для воспроизведения ниже.")}</p>
        <WikipediaCredit url={details.overviewSourceUrl}/>
        {descriptionError&&<p role="status">{descriptionError}</p>}
      </div>
    </div>
    <div className="file-dialog-body">
      <h3>Выберите файл для воспроизведения</h3><p className="file-count">{files.length} видеофайлов · отметка появляется при первом открытии</p>
      {failure&&<p role="alert" className="playback-error">{failure}</p>}
      {busy?<p role="status">Загружаем список файлов…</p>:error?<div role="alert"><p>{error}</p><button onClick={onRetry}>Повторить</button></div>:choice?<section className="play-choice" aria-label="Варианты запуска">
        <h3>{choice.file.name}</h3>
        {choice.active.length?<><p>MPV уже открыт. Заменить файл в одном из окон или открыть отдельное?</p>
          {choice.active.map(session=><button key={session.id} disabled={working} onClick={()=>void play(choice.file,choice.mode,"replace",session.id)}>Заменить: {session.fileName||"Окно MPV"}</button>)}
          <button disabled={working} onClick={()=>void play(choice.file,choice.mode,"new")}>Открыть отдельное окно</button>
        </>:<><p>{choice.file.resumeSeconds>0?"Сохранённая позиция: "+clock(choice.file.resumeSeconds):"Этот файл ещё не начат"}</p>
          {choice.file.resumeSeconds>0&&<button className="primary-action" disabled={working} onClick={()=>void play(choice.file,"resume")}>Продолжить с {clock(choice.file.resumeSeconds)}</button>}
          <button disabled={working} onClick={()=>void play(choice.file,"start")}>Начать сначала</button>
        </>}
        <label className="autonext-option"><input type="checkbox" checked={autoNext} disabled={working} onChange={event=>setAutoNext(event.target.checked)}/>Автоматически открыть следующий файл после окончания</label>
        <button disabled={working} onClick={()=>{setChoice(null);setFailure("")}}>Назад к списку</button>
        {working&&<p role="status">Передаём команду MPV…</p>}
      </section>:files.length?Object.entries(groups).map(([name,items])=><section className="file-group" key={name}><h3>{name}</h3>{items.map(file=><article className="file-row" key={file.id}>
        <button className="file-open" disabled={working} onClick={()=>{setChoice({file,mode:"resume",active:[]});setFailure("")}}>
          <Play size={18} weight="fill"/><span><strong>{file.name}</strong><small>{size(file.length)} · {clock(file.resumeSeconds)} / {file.durationSeconds>0?clock(file.durationSeconds):"длительность ещё неизвестна"}</small></span>
          {file.viewed&&<span className="file-viewed"><Check size={14}/>Просмотрено</span>}
        </button>
        {file.progress!==null&&<progress max={100} value={file.progress} aria-label={"Прогресс: "+file.name}/>}
        {(file.launchCount>0||file.resumeSeconds>0)&&<div className="file-history-actions">
          {file.launchCount>0&&<button disabled={working} onClick={()=>void updateHistory(file,"viewed")}>{file.viewed?"Снять отметку «Просмотрено»":"Отметить просмотренным"}</button>}
          {file.resumeSeconds>0&&<button disabled={working} onClick={()=>void updateHistory(file,"resetPosition")}>Сбросить позицию</button>}
        </div>}
      </article>)}</section>):<p>В торренте нет поддерживаемых видеофайлов.</p>}
    </div>
  </section>;
}

export function WikipediaCredit({url}:{url?:string|null}){
  if(!url||!/^https:\/\/ru\.wikipedia\.org\/\?curid=\d+$/.test(url))return null;
  return <small><a href={url} target="_blank" rel="noopener noreferrer">Описание: Wikipedia</a> · <a href="https://creativecommons.org/licenses/by-sa/4.0/" target="_blank" rel="noopener noreferrer">CC BY-SA 4.0</a></small>;
}

export function PlaybackBar(){
  const [sessions,setSessions]=useState<Session[]>([]);
  const [failure,setFailure]=useState("");
  const [busy,setBusy]=useState(false);
  const locked=useRef(false);
  useEffect(()=>{
    let disposed=false;let pending=false;
    async function poll(){if(pending)return;pending=true;try{const data=await request("/api/mpv/sessions");if(!disposed)setSessions(data.sessions)}catch{if(!disposed)setSessions([])}finally{pending=false}}
    void poll();const timer=window.setInterval(()=>void poll(),1500);
    return()=>{disposed=true;window.clearInterval(timer)};
  },[]);
  async function control(session:Session,action:string,value?:boolean){
    if(locked.current)return;
    locked.current=true;setBusy(true);setFailure("");
    try{const data=await request("/api/mpv/sessions/"+session.id,{action,value});setSessions(data.sessions)}
    catch(error){setFailure(error instanceof Error?error.message:"Не удалось управлять MPV")}
    finally{locked.current=false;setBusy(false)}
  }
  if(!sessions.length&&!failure)return null;
  return <section className="playback-sessions" aria-label="Окна MPV">
    {failure&&<p role="alert" className="playback-error">{failure}</p>}
    {sessions.map(session=><article key={session.id}>
      <div className="session-title"><span>MPV · {session.loading?"Открываем поток…":session.ended?"Воспроизведение завершено":clock(session.timecode)+" / "+(session.duration>0?clock(session.duration):"—")}</span><strong>{session.fileName}</strong>{session.error&&<p role="alert">{session.error}</p>}</div>
      <div className="session-actions">
        <button disabled={busy} onClick={()=>void control(session,"focus")}>Показать окно</button>
        <button disabled={busy||session.loading||!session.canNext} title={session.nextName??"Последний файл"} onClick={()=>void control(session,"next")}>Следующий файл</button>
        <label><input type="checkbox" checked={session.autoNext??false} disabled={busy||session.loading} onChange={event=>void control(session,"autoNext",event.target.checked)}/>Автопереход</label>
        <button disabled={busy} onClick={()=>void control(session,"stop")}>Закрыть MPV</button>
      </div>
    </article>)}
  </section>;
}
