export const RUTUBE_STORAGE = "pirate-cinema-mobile-rutube";
export function rutubeVideo(value) {
  const url = new URL(String(value).trim());
  if (url.protocol !== "https:" || !["rutube.ru", "www.rutube.ru"].includes(url.hostname) || url.username || url.password || url.port) throw new Error("Нужна HTTPS-ссылка на ролик rutube.ru");
  const match = url.pathname.match(/^\/(?:video|shorts|play\/embed)\/([a-f\d]{32})\/?$/i);
  if (!match) throw new Error("Не найден ID ролика RuTube");
  const id=match[1].toLowerCase();
  return {id,url:`https://rutube.ru/video/${id}/`};
}
export function savedRutubeVideos(value) {
  if (!Array.isArray(value)) return [];
  const seen=new Set(), seconds=value=>typeof value==="number"&&Number.isFinite(value)?Math.max(0,value):0;
  return value.slice(0,1000).flatMap(raw=>{try{const video=rutubeVideo(raw.url);if(seen.has(video.id))return [];seen.add(video.id);return [{...video,title:String(raw.title||"RuTube").slice(0,300),position:seconds(raw.position),duration:seconds(raw.duration),completed:raw.completed===true}]}catch{return []}});
}
export function rutubeProgress(previous,message) {
  let event;try{event=typeof message==="string"?JSON.parse(message):message}catch{return previous}
  const valid=value=>typeof value==="number"&&Number.isFinite(value)&&value>=0;
  if(event?.type==="player:currentTime"&&valid(event.data?.time))return {...previous,position:event.data.time,completed:false};
  if(event?.type==="player:durationChange"&&valid(event.data?.duration))return {...previous,duration:event.data.duration};
  if(event?.type==="player:playComplete")return {...previous,position:0,completed:true};
  return previous;
}
