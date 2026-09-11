import {matchConfidence, parseTorrentTitle} from "./title-parser.mjs";

async function query(language, params) {
  const url = new URL("https://" + language + ".wikipedia.org/w/api.php");
  for (const [key, value] of Object.entries({action:"query",format:"json",formatversion:"2",...params})) url.searchParams.set(key, value);
  const response = await fetch(url, {headers:{accept:"application/json","user-agent":"PirateCinema/0.3.2 (local desktop media library)"},signal:AbortSignal.timeout(10000)});
  if (!response.ok) throw new Error("Wikipedia " + response.status);
  const payload = await response.json();
  if (payload.error) throw new Error("Wikipedia: " + payload.error.code);
  return payload.query?.pages ?? [];
}
const extracts = {prop:"pageimages|extracts|pageprops",piprop:"thumbnail",pithumbsize:"600",exintro:"1",explaintext:"1",exlimit:"max"};

function rank(pages, aliases, year, series) {
  return pages.flatMap(page => {
    if (page.pageprops?.disambiguation !== undefined || !page.extract) return [];
    const intro = page.extract.split("\n")[0];
    const text = page.title + " " + intro;
    const tv = /телесериал|мультсериал|мини-сериал|(?:телевизионн|анимационн).{0,15}сериал|television series|tv series|miniseries/i.test(text);
    if (!tv && !/фильм|film|movie/i.test(text)) return [];
    if (series && !tv) return [];
    const title = page.title.replace(/\s*\([^)]*\)\s*$/, "");
    const candidateYear = Number(page.title.match(/(?:19|20)\d{2}/)?.[0] ?? intro.match(/(?:19|20)\d{2}/)?.[0]) || null;
    // Season release dates differ from a show's premiere; films must match their release year.
    if (!tv && year && candidateYear && Math.abs(year-candidateYear)>1) return [];
    const original = intro.match(/(?:англ\.|English:)\s*([^;)]+)/i)?.[1]?.replace(/[«»"“”]/g,"").trim();
    const confidence = Math.max(...aliases.flatMap(alias => [title, original].filter(Boolean).map(name =>
      matchConfidence({title:alias,year:tv?null:year},{title:name,year:candidateYear,type:tv?"tvSeries":"movie"}))));
    return confidence >= .78 ? [{page,title,year:candidateYear,type:tv?"tv":"movie",confidence}] : [];
  }).sort((a,b)=>b.confidence-a.confidence);
}

export async function findWikipedia(input, series=false) {
  const parsed = parseTorrentTitle(input);
  const aliases = [...new Set(input.split(/\s+\/\s+/).map(part => parseTorrentTitle(part.replace(/(?:сезон|сезоны|серии)\s*[:\d].*$/i,"")).title).filter(Boolean))];
  aliases.sort((a,b)=>Number(/[а-яё]/i.test(b))-Number(/[а-яё]/i.test(a)));
  const year = parsed.year ?? (Number(input.match(/(?:19|20)\d{2}/)?.[0]) || null);
  let best;
  // Search localized and original names, with a bounded request count.
  for (const alias of aliases.slice(0,2)) {
    const pages = await query("ru", {...extracts,generator:"search",gsrnamespace:"0",gsrlimit:"8",gsrsearch:alias + (series?" сериал":" фильм")});
    best = rank(pages,aliases,year,series)[0];
    if (best) break;
  }
  if (!best) {
    const english = aliases.find(alias=>/^[\x20-\x7e]+$/.test(alias));
    if (english) {
      const pages = await query("en", {...extracts,prop:extracts.prop+"|langlinks",lllang:"ru",generator:"search",gsrnamespace:"0",gsrlimit:"5",gsrsearch:english+(series?" television series":" film")});
      const match = rank(pages,[english],year,series)[0];
      const russianTitle = match?.page.langlinks?.find(link=>link.lang==="ru")?.title;
      if (russianTitle) {
        const translated = await query("ru", {...extracts,titles:russianTitle,redirects:"1"});
        best = rank(translated,[russianTitle.replace(/\s*\([^)]*\)\s*$/,"")],year,series)[0];
      }
    }
  }
  if (!best) return null;
  return {provider:"wikipedia",providerId:String(best.page.pageid),title:best.title,originalTitle:parsed.title,
    year:best.year,type:best.type,posterUrl:best.page.thumbnail?.source??null,backdropUrl:null,runtimeSeconds:null,rating:null,genres:[],
    overview:best.page.extract,overviewSourceUrl:"https://ru.wikipedia.org/?curid="+best.page.pageid,gallery:[],confidence:best.confidence};
}
