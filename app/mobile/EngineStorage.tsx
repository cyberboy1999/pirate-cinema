import {Capacitor,registerPlugin} from "@capacitor/core";
import {useEffect,useState} from "react";
const Engine=registerPlugin<{storage(options:{quotaGiB?:number}):Promise<{quotaGiB:number;usedBytes:number;availableBytes:number;warning:string}>}>("Librqbit");
export default function EngineStorage(){
  const [status,setStatus]=useState<{quotaGiB:number;usedBytes:number;availableBytes:number;warning:string}|null>(null),[error,setError]=useState("");
  useEffect(()=>{if(!Capacitor.isNativePlatform())return;let active=true;async function update(){try{const next=await Engine.storage({});if(active)setStatus(next)}catch(reason){if(active)setError(String(reason))}}void update();const timer=setInterval(update,5000);return()=>{active=false;clearInterval(timer)}},[]);
  if(!Capacitor.isNativePlatform())return null;
  return <section aria-label="Хранилище торрентов"><h3>Хранилище торрентов</h3>{status&&<><p>Занято {(status.usedBytes/1024**3).toFixed(2)} ГБ · свободно {(status.availableBytes/1024**3).toFixed(2)} ГБ</p><label>Лимит данных<select value={status.quotaGiB} onChange={event=>{void Engine.storage({quotaGiB:Number(event.target.value)}).then(setStatus).catch(reason=>setError(String(reason)))}}>{[1,2,4,8,16,32,64,128].map(value=><option key={value} value={value}>{value} ГБ</option>)}</select></label>{status.warning&&<p role="alert">{status.warning}</p>}</>}{error&&<p role="alert">{error}</p>}</section>;
}
