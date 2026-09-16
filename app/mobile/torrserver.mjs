const VIDEO=/\.(?:mkv|mp4|avi|mov|m4v|webm|ts|m2ts)$/i;

export function torrentFiles(raw){
  let data=raw?.data;
  if(typeof data==="string")try{data=JSON.parse(data)}catch{data={}}
  const files=raw?.file_stats??raw?.files??data?.TorrServer?.Files??[];
  return files.map((file,index)=>{const path=String(file.path??file.name??`Файл ${index+1}`);return {id:Number(file.id??file.index??index+1),path,name:path.split(/[\\/]/).pop()||path,length:Number(file.length??file.size??0)}}).filter(file=>VIDEO.test(file.name));
}

export function normalizeTorrent(raw){
  const hash=String(raw?.hash??raw?.Hash??raw?.info_hash??"").toLowerCase();
  return {hash,title:String((raw?.title??raw?.name??hash)||"Без названия"),poster:String(raw?.poster??""),files:torrentFiles(raw)};
}

export function streamUrl(baseUrl,hash,file){
  return `${baseUrl.replace(/\/$/,"")}/stream/${encodeURIComponent(file.name)}?link=${encodeURIComponent(hash)}&index=${file.id}&play`;
}

export function mpvIntent(url,title){
  const parsed=new URL(url);const scheme=parsed.protocol.slice(0,-1);
  return `intent://${parsed.host}${parsed.pathname}${parsed.search}#Intent;scheme=${scheme};type=video/any;package=is.xyz.mpv;S.title=${encodeURIComponent(title)};S.browser_fallback_url=${encodeURIComponent(url)};end`;
}
