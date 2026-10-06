import test from "node:test";
import assert from "node:assert/strict";
import { PublicMetadataService,createMetadataService,findWikidata } from "../server/metadata-service.mjs";

test("keeps public metadata opt-in",()=>{
  assert.equal(createMetadataService({}).mode,"local");
  assert.equal(createMetadataService({REMOTE_POSTERS:"1"}).mode,"public");
});

test("loads poster and details from Cinemeta without a key",async()=>{
  const originalFetch=globalThis.fetch;globalThis.fetch=async value=>{const url=String(value);if(url.includes("/catalog/movie/"))return new Response(JSON.stringify({metas:[{id:"tt1160419",name:"Dune",type:"movie",releaseInfo:"2021",poster:"https://images.example/dune.jpg"}]}));if(url.includes("/catalog/series/"))return new Response(JSON.stringify({metas:[]}));if(url.includes("/meta/movie/tt1160419"))return new Response(JSON.stringify({meta:{id:"tt1160419",imdb_id:"tt1160419",name:"Dune",type:"movie",year:"2021",poster:"https://images.example/dune.jpg",runtime:"155 min",imdbRating:"8.0",genres:["Sci-Fi"],description:"Arrakis"}}));if(url.includes("query.wikidata.org"))return new Response(JSON.stringify({results:{bindings:[]}}));throw new Error(`Unexpected URL: ${url}`)};
  try{const metadata=await new PublicMetadataService().findBestMatch("Dune.2021.1080p");assert.equal(metadata?.provider,"cinemeta");assert.equal(metadata?.providerId,"tt1160419");assert.equal(metadata?.runtimeSeconds,9300);assert.equal(metadata?.posterUrl,"https://images.example/dune.jpg")}finally{globalThis.fetch=originalFetch}
});

test("uses Wikidata only as an IMDb-linked metadata fallback",async()=>{
  const originalFetch=globalThis.fetch;globalThis.fetch=async value=>new Response(JSON.stringify({results:{bindings:[{label:{value:"Дюна"},description:{value:"Научная фантастика"},image:{value:"https://commons.wikimedia.org/wiki/Special:FilePath/Dune_poster.jpg"}}]}}));
  try{const item=await findWikidata("tt1160419");assert.equal(item?.title,"Дюна");assert.equal(item?.posterUrl,"https://www.wikidata.org/wiki/Special:FilePath/Dune%20poster.jpg");assert.equal(await findWikidata("not-imdb"),null)}finally{globalThis.fetch=originalFetch}
});
