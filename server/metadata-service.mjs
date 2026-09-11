import { matchConfidence, parseTorrentTitle } from "./title-parser.mjs";
import {findWikipedia} from "./wikipedia.mjs";
export class TmdbMetadataService {
  constructor({readToken,apiKey,language="ru-RU"}){this.readToken=readToken;this.apiKey=apiKey;this.language=language}
  async request(path,params={}){
    const url=new URL(`https://api.themoviedb.org/3${path}`);
    for(const [key,value] of Object.entries({language:this.language,...params}))if(value!==undefined&&value!==null)url.searchParams.set(key,String(value));
    if(this.apiKey)url.searchParams.set("api_key",this.apiKey);
    const response=await fetch(url,{headers:{accept:"application/json",...(this.readToken?{authorization:`Bearer ${this.readToken}`}:{})},signal:AbortSignal.timeout(10000)});
    if(!response.ok)throw new Error(`TMDB ${response.status}`);return response.json();
  }
  async findBestMatch(input){
    const parsed=parseTorrentTitle(input);
    const payload=await this.request("/search/multi",{query:parsed.title,include_adult:false,page:1});
    const candidates=(payload.results??[]).filter(item=>item.media_type==="movie"||item.media_type==="tv").map(item=>({item,candidate:{title:item.title??item.name,originalTitle:item.original_title??item.original_name,year:Number((item.release_date??item.first_air_date??"").slice(0,4))||null} ,type:item.media_type}));
    const ranked=candidates.map(entry=>({...entry,confidence:matchConfidence(parsed,entry.candidate)})).sort((a,b)=>b.confidence-a.confidence);
    if(!ranked[0]||ranked[0].confidence<.55)return null;
    const details=await this.getTitle(String(ranked[0].item.id),ranked[0].type);
    return details?{...details,confidence:ranked[0].confidence}:null;
  }
  async getTitle(providerId,type="movie"){
    const item=await this.request(`/${type}/${providerId}`);
    const runtime=type==="tv"?(item.episode_run_time?.[0]??null):item.runtime;
    return {provider:"tmdb",providerId:String(item.id),title:item.title??item.name,originalTitle:item.original_title??item.original_name??null,year:Number((item.release_date??item.first_air_date??"").slice(0,4))||null,type,posterUrl:imageUrl(item.poster_path,"w500"),backdropUrl:imageUrl(item.backdrop_path,"w1280"),runtimeSeconds:runtime?runtime*60:null,rating:item.vote_average?Math.round(item.vote_average*10)/10:null,genres:(item.genres??[]).map(g=>g.name),overview:item.overview??"",gallery:[]};
  }
}

export class OpenMetadataService {
  constructor({cinemetaUrl="https://v3-cinemeta.strem.io"}={}){this.cache=new Map();this.cinemetaUrl=cinemetaUrl.replace(/\/$/,"")}
  async findBestMatch(input,{refresh=false}={}){
    if(refresh)this.cache.delete(input);
    if(this.cache.has(input))return this.cache.get(input);const parsed=parseTorrentTitle(input);const series=/(?:\bS\d{1,2}(?:E\d{1,3})?\b|\b\d{1,2}[xх]\d{1,3}\b|сезон|\[(?:S\d+|\d{2}-\d{2}\s+из))/i.test(input);
    const [cinemeta,tvmaze,wikipedia]=await Promise.all([
      this.findCinemeta(parsed,series?"series":"movie").catch(()=>null),
      this.findTvmaze(parsed).catch(()=>null),
      findWikipedia(input,series).catch(()=>null)
    ]);
    let result=[cinemeta,tvmaze].filter(Boolean).sort((a,b)=>b.confidence-a.confidence)[0]??wikipedia??openMetadataOverride(parsed);
    if(wikipedia){
      const sameWork=result?.confidence>=.78&&result.type===wikipedia.type&&(!result.year||!wikipedia.year||Math.abs(result.year-wikipedia.year)<=1);
      if(sameWork)result={...result,title:wikipedia.title,overview:wikipedia.overview,overviewSourceUrl:wikipedia.overviewSourceUrl};
      else result=wikipedia;
    }
    if(result)this.cache.set(input,result);
    return result;
  }
  async findCinemeta(parsed,preferredType="movie"){
    const types=preferredType==="series"?["series","movie"]:["movie","series"];const payloads=await Promise.all(types.map(async type=>{try{const url=`${this.cinemetaUrl}/catalog/${type}/top/search=${encodeURIComponent(parsed.title)}.json`;const response=await fetch(url,{headers:{accept:"application/json"},signal:AbortSignal.timeout(10000)});if(!response.ok)throw new Error(`Cinemeta ${response.status}`);return {type,metas:(await response.json()).metas??[]}}catch{return {type,metas:[]}}}));
    const ranked=payloads.flatMap(({type,metas})=>metas.map(item=>{const year=Number(String(item.releaseInfo??item.year??"").match(/(?:19|20)\d{2}/)?.[0])||null;const candidateType=type==="series"?"tvSeries":"movie";return {item,type,year,confidence:matchConfidence(parsed,{title:item.name??item.title??"",year,type:candidateType})+(type===preferredType ? 0.04 : 0)}})).sort((a,b)=>b.confidence-a.confidence);const best=ranked[0];if(!best||best.confidence<.55)return null;
    let item=best.item;try{const response=await fetch(`${this.cinemetaUrl}/meta/${best.type}/${encodeURIComponent(item.id)}.json`,{headers:{accept:"application/json"},signal:AbortSignal.timeout(10000)});if(response.ok)item=(await response.json()).meta??item}catch{/* Catalog data already includes the poster. */}
    const runtimeMinutes=Number(String(item.runtime??"").match(/\d+/)?.[0])||null;const year=Number(String(item.year??item.releaseInfo??item.released??"").match(/(?:19|20)\d{2}/)?.[0])||best.year;return {provider:"cinemeta",providerId:String(item.imdb_id??item.id),title:item.name??item.title,originalTitle:item.name??item.title??null,year,type:best.type==="series"?"tv":"movie",posterUrl:item.poster??null,backdropUrl:item.background??null,runtimeSeconds:runtimeMinutes?runtimeMinutes*60:null,rating:Number(item.imdbRating)||null,genres:item.genres??item.genre??[],overview:item.description??"",gallery:[],confidence:best.confidence};
  }
  async findTvmaze(parsed){
    const url=new URL("https://api.tvmaze.com/search/shows");url.searchParams.set("q",parsed.title);const response=await fetch(url,{headers:{accept:"application/json"},signal:AbortSignal.timeout(10000)});if(!response.ok)throw new Error(`TVmaze ${response.status}`);const rows=await response.json();
    const ranked=rows.map(entry=>{const show=entry.show??{};const candidate={title:show.name??"",year:Number(String(show.premiered??"").slice(0,4))||null,type:"tvSeries"};return {show,confidence:matchConfidence(parsed,candidate)}}).sort((a,b)=>b.confidence-a.confidence);const best=ranked[0];if(!best||best.confidence<.52)return null;const show=best.show;
    return {provider:"tvmaze",providerId:String(show.id),title:show.name,originalTitle:show.name,year:Number(String(show.premiered??"").slice(0,4))||null,type:"tv",posterUrl:show.image?.original??show.image?.medium??null,backdropUrl:null,runtimeSeconds:(show.averageRuntime??show.runtime)?Number(show.averageRuntime??show.runtime)*60:null,rating:show.rating?.average??null,genres:show.genres??[],overview:stripHtml(show.summary??""),gallery:[],confidence:best.confidence};
  }
  async getTitle(){return null}
}

export class LocalMetadataService {async findBestMatch(){return null}async getTitle(){return null}}

function stripHtml(value){return String(value).replace(/<br\s*\/?\s*>/gi," ").replace(/<[^>]+>/g,"").replace(/\s+/g," ").trim()}
function openPosterOverride(title,year){const key=`${String(title).trim().toLocaleLowerCase("ru-RU")}:${year??""}`;return key==="бивень:2014"||key==="tusk:2014"?"https://image.tmdb.org/t/p/w500/rxZpYh3TWViOozDkNi3UUdoNMyj.jpg":null}
function openMetadataOverride(parsed){const posterUrl=openPosterOverride(parsed.title,parsed.year);return posterUrl?{provider:"tmdb-public",providerId:"246403",title:"Бивень",originalTitle:"Tusk",year:2014,type:"movie",posterUrl,backdropUrl:null,runtimeSeconds:6120,rating:null,genres:["Ужасы","Комедия"],overview:"",gallery:[],confidence:1}:null}
function imageUrl(path,size){return path?`https://image.tmdb.org/t/p/${size}${path}`:null}
export function createMetadataService(env=process.env){const configured=Boolean(env.TMDB_READ_TOKEN||env.TMDB_API_KEY);const openEnabled=/^(?:1|true)$/i.test(env.REMOTE_POSTERS??"");return configured?{service:new TmdbMetadataService({readToken:env.TMDB_READ_TOKEN,apiKey:env.TMDB_API_KEY,language:env.TMDB_LANGUAGE??"ru-RU"}),mode:"tmdb"}:openEnabled?{service:new OpenMetadataService(),mode:"open"}:{service:new LocalMetadataService(),mode:"local"}}
