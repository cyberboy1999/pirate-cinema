import { matchConfidence, parseTorrentTitle } from "./title-parser.mjs";

const CINEMETA = "https://v3-cinemeta.strem.io";
const WIKIDATA = "https://query.wikidata.org/sparql";

export class PublicMetadataService {
  constructor({cinemetaUrl=CINEMETA}={}){this.cache=new Map();this.cinemetaUrl=cinemetaUrl.replace(/\/$/,"")}
  async findBestMatch(input,{refresh=false}={}){
    if(refresh)this.cache.delete(input);
    if(this.cache.has(input))return this.cache.get(input);
    const parsed=parseTorrentTitle(input),series=/(?:\bS\d{1,2}(?:E\d{1,3})?\b|\b\d{1,2}[xх]\d{1,3}\b|сезон|\[(?:S\d+|\d{2}-\d{2}\s+из))/i.test(input);
    const [cinemeta,tvmaze]=await Promise.all([this.findCinemeta(parsed,series?"series":"movie").catch(()=>null),this.findTvmaze(parsed).catch(()=>null)]);
    let result=[cinemeta,tvmaze].filter(Boolean).sort((a,b)=>b.confidence-a.confidence)[0]??null;
    if(result?.providerId?.startsWith("tt")){
      const wiki=await findWikidata(result.providerId).catch(()=>null);
      if(wiki)result={...result,title:wiki.title??result.title,overview:result.overview||wiki.overview,posterUrl:result.posterUrl??wiki.posterUrl,overviewSourceUrl:wiki.sourceUrl};
    }
    if(result)this.cache.set(input,result);
    return result;
  }
  async findCinemeta(parsed,preferredType="movie"){
    const types=preferredType==="series"?["series","movie"]:["movie","series"];
    const payloads=await Promise.all(types.map(async type=>{try{const response=await fetch(`${this.cinemetaUrl}/catalog/${type}/top/search=${encodeURIComponent(parsed.title)}.json`,{headers:{accept:"application/json"},signal:AbortSignal.timeout(10000)});if(!response.ok)throw new Error(`Cinemeta ${response.status}`);return {type,metas:(await response.json()).metas??[]}}catch{return {type,metas:[]}}}));
    const ranked=payloads.flatMap(({type,metas})=>metas.map(item=>{const year=Number(String(item.releaseInfo??item.year??"").match(/(?:19|20)\d{2}/)?.[0])||null;return {item,type,year,confidence:matchConfidence(parsed,{title:item.name??item.title??"",year,type:type==="series"?"tvSeries":"movie"})+(type===preferredType?.04:0)}})).sort((a,b)=>b.confidence-a.confidence),best=ranked[0];
    if(!best||best.confidence<.55)return null;
    let item=best.item;try{const response=await fetch(`${this.cinemetaUrl}/meta/${best.type}/${encodeURIComponent(item.id)}.json`,{headers:{accept:"application/json"},signal:AbortSignal.timeout(10000)});if(response.ok)item=(await response.json()).meta??item}catch{}
    const runtimeMinutes=Number(String(item.runtime??"").match(/\d+/)?.[0])||null,year=Number(String(item.year??item.releaseInfo??item.released??"").match(/(?:19|20)\d{2}/)?.[0])||best.year;
    return {provider:"cinemeta",providerId:String(item.imdb_id??item.id),title:item.name??item.title,originalTitle:item.name??item.title??null,year,type:best.type==="series"?"tv":"movie",posterUrl:item.poster??null,backdropUrl:item.background??null,runtimeSeconds:runtimeMinutes?runtimeMinutes*60:null,rating:Number(item.imdbRating)||null,genres:item.genres??item.genre??[],overview:item.description??"",gallery:[],confidence:best.confidence};
  }
  async findTvmaze(parsed){
    const url=new URL("https://api.tvmaze.com/search/shows");url.searchParams.set("q",parsed.title);const response=await fetch(url,{headers:{accept:"application/json"},signal:AbortSignal.timeout(10000)});if(!response.ok)throw new Error(`TVmaze ${response.status}`);const rows=await response.json();
    const ranked=rows.map(entry=>{const show=entry.show??{},candidate={title:show.name??"",year:Number(String(show.premiered??"").slice(0,4))||null,type:"tvSeries"};return {show,confidence:matchConfidence(parsed,candidate)}}).sort((a,b)=>b.confidence-a.confidence),best=ranked[0];if(!best||best.confidence<.52)return null;const show=best.show;
    return {provider:"tvmaze",providerId:String(show.id),title:show.name,originalTitle:show.name,year:Number(String(show.premiered??"").slice(0,4))||null,type:"tv",posterUrl:show.image?.original??show.image?.medium??null,backdropUrl:null,runtimeSeconds:(show.averageRuntime??show.runtime)?Number(show.averageRuntime??show.runtime)*60:null,rating:show.rating?.average??null,genres:show.genres??[],overview:stripHtml(show.summary??""),gallery:[],confidence:best.confidence};
  }
  async getTitle(){return null}
}

export class LocalMetadataService {async findBestMatch(){return null}async getTitle(){return null}}

export async function findWikidata(imdbId){
  if(!/^tt\d{1,12}$/.test(imdbId))return null;
  const query=`SELECT ?label ?description ?image WHERE { ?item <http://www.wikidata.org/prop/direct/P345> "${imdbId}". OPTIONAL { ?item <http://www.w3.org/2000/01/rdf-schema#label> ?label. FILTER(LANG(?label)="ru") } OPTIONAL { ?item <http://schema.org/description> ?description. FILTER(LANG(?description)="ru") } OPTIONAL { ?item <http://www.wikidata.org/prop/direct/P18> ?image. } } LIMIT 1`;
  const response=await fetch(`${WIKIDATA}?format=json&query=${encodeURIComponent(query)}`,{headers:{accept:"application/sparql-results+json"},signal:AbortSignal.timeout(10000)});if(!response.ok)throw new Error(`Wikidata ${response.status}`);const row=(await response.json()).results?.bindings?.[0];if(!row)return null;
  const file=row.image?.value?.split("/").pop()?.replaceAll("_"," ");return {title:row.label?.value??null,overview:row.description?.value??null,posterUrl:file?`https://www.wikidata.org/wiki/Special:FilePath/${encodeURIComponent(file)}`:null,sourceUrl:"https://www.wikidata.org/"};
}

function stripHtml(value){return String(value).replace(/<br\s*\/?\s*>/gi," ").replace(/<[^>]+>/g,"").replace(/\s+/g," ").trim()}
export function createMetadataService(env=process.env){const enabled=/^(?:1|true)$/i.test(env.REMOTE_POSTERS??"");return enabled?{service:new PublicMetadataService(),mode:"public"}:{service:new LocalMetadataService(),mode:"local"}}
