import test from "node:test";
import assert from "node:assert/strict";
import { OpenMetadataService } from "../server/metadata-service.mjs";

test("loads posters and details from Cinemeta",async()=>{
  const originalFetch=globalThis.fetch;globalThis.fetch=async value=>{const url=String(value);if(url.includes("/catalog/movie/"))return new Response(JSON.stringify({metas:[{id:"tt1160419",name:"Dune",type:"movie",releaseInfo:"2021",poster:"https://images.example/dune-small.jpg"}]}));if(url.includes("/catalog/series/"))return new Response(JSON.stringify({metas:[]}));if(url.includes("/meta/movie/tt1160419"))return new Response(JSON.stringify({meta:{id:"tt1160419",imdb_id:"tt1160419",name:"Dune",type:"movie",year:"2021",poster:"https://images.metahub.space/poster/small/tt1160419/img",runtime:"155 min",imdbRating:"8.0",genres:["Sci-Fi"],description:"Arrakis"}}));throw new Error(`Unexpected URL: ${url}`)};
  try{const metadata=await new OpenMetadataService().findBestMatch("Dune.2021.1080p");assert.equal(metadata?.provider,"cinemeta");assert.equal(metadata?.providerId,"tt1160419");assert.equal(metadata?.runtimeSeconds,9300);assert.match(metadata?.posterUrl??"",/^https:\/\/images\.metahub\.space\/poster\//)}finally{globalThis.fetch=originalFetch}
});
