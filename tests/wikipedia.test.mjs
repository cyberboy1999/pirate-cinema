import test from "node:test";
import assert from "node:assert/strict";
import {findWikipedia} from "../server/wikipedia.mjs";
import {OpenMetadataService} from "../server/metadata-service.mjs";
import {MediaDatabase} from "../server/database.mjs";

const page={pageid:123,title:"Дюна (фильм, 2021)",extract:"Дюна (англ. Dune) — научно-фантастический фильм 2021 года. История путешествия на далёкую планету."};
function mock(t,handler){t.mock.method(globalThis,"fetch",async value=>new Response(JSON.stringify(await handler(new URL(value)))));}

test("Russian Wikipedia description wins while Cinemeta poster is retained",async t=>{
  const searches=[];
  mock(t,url=>{
    if(url.hostname==="ru.wikipedia.org"){searches.push(url.searchParams.get("gsrsearch"));return {query:{pages:[page]}}}
    if(url.pathname.includes("/catalog/movie/"))return {metas:[{id:"tt1",name:"Dune",year:2021,poster:"https://example.org/poster.jpg",description:"English description"}]};
    if(url.pathname.includes("/catalog/series/"))return {metas:[]};
    if(url.pathname.includes("/meta/"))return {};
    return [];
  });
  const result=await new OpenMetadataService().findBestMatch("Дюна / Dune.2021.1080p");
  assert.equal(searches[0],"Дюна фильм");
  assert.equal(result.provider,"cinemeta");
  assert.equal(result.posterUrl,"https://example.org/poster.jpg");
  assert.equal(result.overview,page.extract);
  assert.equal(result.overviewSourceUrl,"https://ru.wikipedia.org/?curid=123");
});

test("English-only names can follow a Russian language link",async t=>{
  mock(t,url=>{
    if(url.hostname==="en.wikipedia.org")return {query:{pages:[{pageid:5,title:"Dune (2021 film)",extract:"Dune is a 2021 science fiction film.",langlinks:[{lang:"ru",title:page.title}]}]}};
    return {query:{pages:url.searchParams.has("titles")?[page]:[]}};
  });
  assert.equal((await findWikipedia("Dune.2021.mkv")).overview,page.extract);
});

test("rejects disambiguation pages, unrelated works and wrong film years",async t=>{
  mock(t,()=>({query:{pages:[
    {...page,title:"Дюна (фильм, 1984)",extract:"Дюна — фильм 1984 года."},
    {...page,title:"Дюна",extract:"Дюна — роман 1965 года."},
    {...page,pageprops:{disambiguation:""}},
    {...page,title:"Другой фильм (2021)",extract:"Другой фильм — фильм 2021 года."}
  ]}}));
  assert.equal(await findWikipedia("Дюна.2021.mkv"),null);
});

test("series query ignores season year and excludes same-name films",async t=>{
  mock(t,url=>{
    assert.equal(url.searchParams.get("gsrsearch"),"Мажор сериал");
    return {query:{pages:[
      {pageid:1,title:"Мажор (фильм)",extract:"Мажор — фильм 2021 года."},
      {pageid:2,title:"Мажор (телесериал)",extract:"Мажор — российский телесериал 2014 года."}
    ]}};
  });
  const result=await findWikipedia("Мажор Сезон 4 (2022)",true);
  assert.equal(result.providerId,"2");
  assert.equal(result.type,"tv");
});

test("recognizes animation series without confusing them with feature films",async t=>{
  mock(t,()=>({query:{pages:[{pageid:3,title:"Смешарики",extract:"Смешарики — российский анимационный сериал о приключениях друзей, выходящий с 2004 года."}]}}));
  assert.equal((await findWikipedia("Смешарики",true)).type,"tv");
});

test("uses Wikipedia original title to recover a Cinemeta poster for localized series",async t=>{
  const silo={pageid:9,title:"Укрытие (телесериал)",extract:"«Укрытие» (англ. Silo) — американский научно-фантастический телесериал, премьера которого состоялась в 2023 году."};
  mock(t,url=>{
    if(url.hostname==="ru.wikipedia.org")return {query:{pages:[silo]}};
    if(url.pathname.includes("search=%D0%A3%D0%BA%D1%80%D1%8B%D1%82%D0%B8%D0%B5"))return {metas:[]};
    if(url.pathname.includes("search=Silo"))return {metas:[{id:"tt14688458",name:"Silo",releaseInfo:"2023",poster:"https://example.org/silo.jpg"}]};
    if(url.pathname.includes("/meta/"))return {meta:{id:"tt14688458",name:"Silo",year:"2023",poster:"https://example.org/silo.jpg"}};
    return {metas:[]};
  });
  const result=await new OpenMetadataService().findBestMatch("Укрытие 2026");
  assert.equal(result.posterUrl,"https://example.org/silo.jpg");assert.equal(result.overview,silo.extract);assert.equal(result.overviewSourceUrl,"https://ru.wikipedia.org/?curid=9");
});

test("SQLite preserves Russian description and source during English fallback",()=>{
  const db=new MediaDatabase(":memory:");
  try{
    db.upsert({torrentHash:"abc",torrentName:"Dune",title:"Dune"});
    db.saveDescription("abc","English description");
    db.saveDescription("abc",page.extract,"https://ru.wikipedia.org/?curid=123");
    db.saveDescription("abc","English fallback");
    assert.equal(db.getMedia("abc").overview,page.extract);
    assert.equal(db.getMedia("abc").overviewSourceUrl,"https://ru.wikipedia.org/?curid=123");
    const checked=db.get("abc").metadata_checked_at;
    db.migrate();
    assert.equal(db.get("abc").metadata_checked_at,checked);
  }finally{db.close()}
});
