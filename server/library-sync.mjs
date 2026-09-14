import {localizedTorrentTitle,parseTorrentTitle} from "./title-parser.mjs";
import {normalizeTorrent,normalizeViewed} from "./torrserver-client.mjs";

export function createLibrarySync({db,client,metadata,cachePoster,state,activeHashes=()=>[]}){
  let running=null,fullRunning=false;
  const descriptions=new Map();
  function item(hash){return db.getMedia(hash)}
  async function enrich(hash,force=false){
    if(descriptions.has(hash))return descriptions.get(hash);
    const existing=db.get(hash);
    if(!existing)return null;
    if(!force&&existing.metadata_checked_at&&Date.now()-Date.parse(existing.metadata_checked_at)<86400000)return item(hash);
    const work=(async()=>{
      try{
        const query=existing.metadata_query??existing.torrent_name;
        const found=await metadata.findBestMatch(query,{refresh:force});
        const accepted=found&&found.confidence>=(found.provider==="tmdb"?.85:.58)?found:null;
        // Re-read after network I/O: sync or playback may have updated this card.
        const current=item(hash);
        if(!current)return null;
        if(accepted){
          const posterPath=accepted.posterUrl?await cachePoster(accepted.provider+"-"+accepted.providerId,accepted.posterUrl):null;
          if(!db.get(hash))return null;
          db.upsert({...current,metadataProvider:accepted.provider,providerId:accepted.providerId,
            title:current.metadataQuery??accepted.title??localizedTorrentTitle(current.torrentName)??current.title,
            originalTitle:accepted.originalTitle,year:accepted.year??current.year,mediaType:accepted.type??current.mediaType,
            posterUrl:accepted.posterUrl??current.posterUrl,posterPath:posterPath??db.get(hash).poster_path,
            runtimeSeconds:accepted.runtimeSeconds??current.runtimeSeconds,rating:accepted.rating??current.rating,
            genres:accepted.genres?.length?accepted.genres:current.genres,matchConfidence:accepted.confidence});
        }
        db.saveDescription(hash,accepted?.overview,accepted?.overviewSourceUrl);
        return item(hash);
      }finally{descriptions.delete(hash)}
    })();
    descriptions.set(hash,work);
    return work;
  }
  async function synchronize(full=false){
    if(running){
      const wasFull=fullRunning;
      await running;
      if(full&&!wasFull)return synchronize(true);
      return;
    }
    fullRunning=full;
    state.syncing=true;state.error=null;
    state.syncProgress={processed:0,total:0,failed:0,full};
    const startedAt=new Date().toISOString();
    running=(async()=>{
      try{
        let echo;
        try{echo=await client.echo()}catch(error){state.online=false;throw error}
        state.online=Boolean(echo.trim());state.serverVersion=echo.trim();
        if(!state.online)throw new Error("TorrServer не ответил");
        const raw=await client.listTorrents();
        if(!Array.isArray(raw))throw new Error("Некорректный список раздач TorrServer");
        const torrents=raw.map(normalizeTorrent).map(t=>({...t,hash:t.hash.toLowerCase()}));
        if(torrents.some(t=>!/^[a-f0-9]{40}$/.test(t.hash)))throw new Error("Некорректный hash в списке TorrServer");
        state.syncProgress.total=torrents.length;
        for(const torrent of torrents){
          const existing=item(torrent.hash),parsed=parseTorrentTitle(torrent.name);
          db.upsert({...existing,torrentHash:torrent.hash,torrentName:torrent.name,
            title:existing?.metadataQuery??localizedTorrentTitle(torrent.name)??existing?.title??parsed.title,
            year:existing?.year??parsed.year,mediaType:existing?.mediaType??"unknown",
            posterUrl:existing?.posterUrl??torrent.poster,posterPath:db.get(torrent.hash)?.poster_path,
            addedAt:existing?.addedAt??torrent.addedAt,torrentStatus:torrent.status,
            downloadSpeed:torrent.downloadSpeed,uploadSpeed:torrent.uploadSpeed,peers:torrent.peers,isActive:torrent.isActive});
        }
        // An incomplete/failed server response must never clear the local library.
        db.reconcile([...torrents.map(t=>t.hash),...activeHashes(),...db.list().filter(row=>row.addedAt>=startedAt).map(row=>row.torrentHash)]);
        try{
          for(const viewed of normalizeViewed(await client.listViewed())){
            const hash=viewed.hash.toLowerCase();
            if(!db.listFileHistory(hash).length)db.updateViewed(hash,viewed);
          }
        }catch{state.error="Раздачи получены, но история TorrServer временно недоступна"}
        let cursor=0;
        // Two concurrent lookups keep the local UI responsive without flooding metadata providers.
        await Promise.all(Array.from({length:Math.min(2,torrents.length)},async()=>{
          while(cursor<torrents.length){
            const torrent=torrents[cursor++];
            try{await enrich(torrent.hash,full)}
            catch{state.syncProgress.failed++;state.error="Часть описаний не обновилась. Сохранённые данные оставлены."}
            finally{state.syncProgress.processed++}
          }
        }));
        state.lastSync=new Date().toISOString();
      }catch(error){state.error=error.message}
      finally{state.syncing=false}
    })();
    try{await running}finally{running=null;fullRunning=false}
  }
  return {synchronize,enrich,item};
}
