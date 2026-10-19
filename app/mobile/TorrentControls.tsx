import {useEffect,useState} from "react";
import EngineStorage from "./EngineStorage.tsx";
export default function TorrentControls({base,hash}:{base:string;hash:string}){
  const [status,setStatus]=useState(""),[error,setError]=useState("");
  useEffect(()=>{const abort=new AbortController();async function update(){try{const response=await fetch(`${base}/torrents/${hash}/stats/v1`,{signal:abort.signal});if(!response.ok)throw new Error(`HTTP ${response.status}`);const data=await response.json();if(!abort.signal.aborted)setStatus(`${data.state} · ${Math.round(100*data.progress_bytes/Math.max(1,data.total_bytes))}% · ${data.live?.download_speed?.human_readable??"0"} · peers ${data.live?.snapshot?.peer_stats?.live??0}`)}catch(reason){if(!abort.signal.aborted)setError(String(reason))}}void update();const timer=setInterval(update,2000);return()=>{abort.abort();clearInterval(timer)}},[base,hash]);
  async function action(command:"pause"|"start"){try{const response=await fetch(`${base}/torrents/${hash}/${command}`,{method:"POST"});if(!response.ok)throw new Error(`HTTP ${response.status}`);setError("")}catch(reason){setError(String(reason))}}
  return <section aria-label="Управление торрент-потоком"><p>{status}</p><button onClick={()=>void action("pause")}>Пауза загрузки</button><button onClick={()=>void action("start")}>Продолжить загрузку</button><EngineStorage/>{error&&<p role="alert">{error}</p>}</section>;
}
