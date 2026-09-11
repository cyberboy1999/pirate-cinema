"use client";

import {useState} from "react";
import {Check,Desktop,FolderOpen,Play} from "@phosphor-icons/react";

export type Preferences={onboardingComplete:boolean;language:"ru"|"en";playerType:"mpv"|"external";playerPath:string|null};

async function chooseExecutable(){
  if(window.pirateCinema)return window.pirateCinema.choosePlayer();
  return window.prompt("Введите полный путь к EXE-файлу плеера")?.trim()||null;
}

export function Welcome({value,onSave}:{value:Preferences;onSave:(next:Preferences)=>Promise<void>}){
  const [draft,setDraft]=useState(value);const [busy,setBusy]=useState(false);const [error,setError]=useState("");const en=draft.language==="en";
  async function selectExternal(){const path=await chooseExecutable();if(path)setDraft(current=>({...current,playerType:"external",playerPath:path}))}
  async function finish(){setBusy(true);setError("");try{await onSave({...draft,onboardingComplete:true})}catch(reason){setError(reason instanceof Error?reason.message:"Не удалось сохранить настройки")}finally{setBusy(false)}}
  return <div className="welcome-backdrop"><section className="welcome-card" role="dialog" aria-modal="true" aria-labelledby="welcome-title">
    <img src="/pirate-cinema-logo.png" alt=""/><span>PIRATE CINEMA</span><h1 id="welcome-title">{en?"Welcome":"Добро пожаловать"}</h1><p>{en?"Choose the interface language and how movies will open.":"Выберите язык интерфейса и способ запуска фильмов."}</p>
    <fieldset><legend>{en?"Interface language":"Язык приложения"}</legend><div className="choice-row"><button className={draft.language==="ru"?"active":""} onClick={()=>setDraft({...draft,language:"ru"})}>Русский</button><button className={draft.language==="en"?"active":""} onClick={()=>setDraft({...draft,language:"en"})}>English</button></div></fieldset>
    <fieldset><legend>{en?"Video player":"Видеоплеер"}</legend><button className={draft.playerType==="mpv"?"player-choice active":"player-choice"} onClick={()=>setDraft({...draft,playerType:"mpv"})}><Play size={24}/><span><strong>{en?"Built-in MPV":"Встроенный MPV"}</strong><small>{en?"Recommended · resume and viewing history":"Рекомендуется · продолжение и история просмотра"}</small></span>{draft.playerType==="mpv"&&<Check/>}</button><button className={draft.playerType==="external"?"player-choice active":"player-choice"} onClick={()=>void selectExternal()}><Desktop size={24}/><span><strong>{en?"Another local player":"Другой локальный плеер"}</strong><small>{draft.playerPath??(en?"Choose a Windows EXE file":"Выберите EXE-файл Windows")}</small></span><FolderOpen/></button></fieldset>
    {error&&<p role="alert" className="preference-error">{error}</p>}<button className="primary-action welcome-finish" disabled={busy||draft.playerType==="external"&&!draft.playerPath} onClick={()=>void finish()}>{busy?(en?"Saving…":"Сохраняем…"):(en?"Start watching":"Начать просмотр")}</button>
  </section></div>
}

export function PlayerPreferences({value,onSave}:{value:Preferences;onSave:(next:Preferences)=>Promise<void>}){
  const [busy,setBusy]=useState(false);const [error,setError]=useState("");const en=value.language==="en";
  async function save(next:Preferences){setBusy(true);setError("");try{await onSave(next)}catch(reason){setError(reason instanceof Error?reason.message:"Не удалось сохранить настройки")}finally{setBusy(false)}}
  async function external(){const path=await chooseExecutable();if(path)await save({...value,playerType:"external",playerPath:path})}
  return <section className="preference-settings"><h2>{en?"Interface and player":"Интерфейс и плеер"}</h2><label>{en?"Language":"Язык"}<select disabled={busy} value={value.language} onChange={event=>void save({...value,language:event.target.value as "ru"|"en"})}><option value="ru">Русский</option><option value="en">English</option></select></label><div><button disabled={busy} className={value.playerType==="mpv"?"active":""} onClick={()=>void save({...value,playerType:"mpv"})}>{en?"Built-in MPV":"Встроенный MPV"}</button><button disabled={busy} className={value.playerType==="external"?"active":""} onClick={()=>void external()}>{value.playerType==="external"&&value.playerPath?(en?"Change local player":"Изменить локальный плеер"):(en?"Choose local player":"Выбрать локальный плеер")}</button></div>{value.playerType==="external"&&<small>{value.playerPath}</small>}<p>{en?"Only built-in MPV can save the exact playback position through IPC.":"Только встроенный MPV сохраняет точную позицию просмотра через IPC."}</p>{error&&<p role="alert" className="preference-error">{error}</p>}</section>
}
