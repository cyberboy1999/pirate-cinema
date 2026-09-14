import test from "node:test";
import assert from "node:assert/strict";
import { createServer } from "node:http";
import { extractInfoHash,normalizeMagnetLink,TorrServerClient,normalizeSearchResult,normalizeTorrent,normalizeTorrentFiles,normalizeViewed } from "../server/torrserver-client.mjs";

test("reads TorrServer health, torrents, and viewed history",async()=>{
  const server=createServer(async(req,res)=>{const chunks=[];for await(const chunk of req)chunks.push(chunk);const body=chunks.length?JSON.parse(Buffer.concat(chunks).toString("utf8")):{};res.setHeader("content-type","application/json");if(req.url==="/echo")return res.end(JSON.stringify("Server online"));if(req.url==="/torrents"&&body.action==="list")return res.end(JSON.stringify([{hash:"abc",title:"Dune.Part.Two.2024.mkv",stat:{active_peers:4,download_speed:1024}}]));if(req.url==="/viewed"&&body.action==="list")return res.end(JSON.stringify({abc:{timecode:7200,duration:9960}}));res.statusCode=404;res.end("{}")});
  await new Promise(resolve=>server.listen(0,"127.0.0.1",resolve));
  try{const address=server.address();const client=new TorrServerClient(`http://127.0.0.1:${address.port}`);assert.equal(await client.echo(),"Server online");const torrent=normalizeTorrent((await client.listTorrents())[0]);assert.equal(torrent.hash,"abc");assert.equal(torrent.peers,4);const [viewed]=normalizeViewed(await client.listViewed());assert.equal(viewed.timecode,7200)}finally{await new Promise(resolve=>server.close(resolve))}
});

test("removes a saved torrent through the MatriX API",async()=>{
  let received=null;const server=createServer(async(req,res)=>{const chunks=[];for await(const chunk of req)chunks.push(chunk);received=JSON.parse(Buffer.concat(chunks).toString("utf8"));res.setHeader("content-type","application/json");res.end("{}");});
  await new Promise(resolve=>server.listen(0,"127.0.0.1",resolve));
  try{const address=server.address();const client=new TorrServerClient(`http://127.0.0.1:${address.port}`);await client.removeTorrent("abc123");assert.deepEqual(received,{action:"rem",hash:"abc123"})}finally{await new Promise(resolve=>server.close(resolve))}
});

test("configures and searches an optional Torznab indexer",async()=>{
  let saved=null;const server=createServer(async(req,res)=>{const chunks=[];for await(const chunk of req)chunks.push(chunk);const body=chunks.length?JSON.parse(Buffer.concat(chunks).toString("utf8")):{};res.setHeader("content-type","application/json");if(req.url==="/settings"&&body.action==="get")return res.end(JSON.stringify({CacheSize:64}));if(req.url==="/settings"&&body.action==="set"){saved=body.sets;return res.end("")}if(req.url?.startsWith("/torznab/search/"))return res.end(JSON.stringify([{Title:"Movie",Hash:"a".repeat(40)}]));res.statusCode=404;res.end("{}")});
  await new Promise(resolve=>server.listen(0,"127.0.0.1",resolve));try{const address=server.address();const client=new TorrServerClient(`http://127.0.0.1:${address.port}`);await client.configureTorznab({host:"http://127.0.0.1:9696/1",key:"secret"});assert.equal(saved.CacheSize,64);assert.equal(saved.EnableTorznabSearch,true);assert.equal(saved.TorznabUrls[0].Key,"secret");assert.equal((await client.searchTorznab("movie"))[0].Title,"Movie")}finally{await new Promise(resolve=>server.close(resolve))}
});

test("normalizes TorrServer search releases",()=>{
  const hash="a".repeat(40);const item=normalizeSearchResult({Title:"Movie.2024.2160p",Size:"18 GB",Seed:42,Peer:3,Magnet:`magnet:?xt=urn:btih:${hash}&amp;dn=Movie`,VideoQuality:305});
  assert.equal(item.quality,"2160p");assert.equal(item.seeders,42);assert.match(item.magnet,/^magnet:/);assert.equal(item.hash,hash);assert.doesNotMatch(item.magnet,/&amp;/);
});

test("builds a magnet when search returns only an info hash",()=>{
  const hash="c".repeat(40);const item=normalizeSearchResult({Title:"Hash only",Hash:hash,Seed:2});
  assert.equal(item.hash,hash);assert.equal(item.magnet,`magnet:?xt=urn:btih:${hash}`);
});

test("normalizes hex and Base32 magnet info hashes",()=>{
  const hex="abcdef0123456789abcdef0123456789abcdef01";const normalized=normalizeMagnetLink(`  "magnet:?xt=urn:btih:${hex.toUpperCase()}&amp;dn=Test%20Movie"  `);
  assert.equal(normalized.hash,hex);assert.equal(normalized.title,"Test Movie");assert.equal(extractInfoHash(`magnet:?xt=urn:btih:${"A".repeat(32)}`),"0".repeat(40));
  assert.throws(()=>normalizeMagnetLink("magnet:?dn=No hash"),/BTIH/);
});

test("adds a normalized magnet and reuses an existing torrent",async()=>{
  const hash="b".repeat(40);let addBody=null;let listed=[];const server=createServer(async(req,res)=>{const chunks=[];for await(const chunk of req)chunks.push(chunk);const body=chunks.length?JSON.parse(Buffer.concat(chunks).toString("utf8")):{};res.setHeader("content-type","application/json");if(body.action==="list")return res.end(JSON.stringify(listed));if(body.action==="add"){addBody=body;listed=[{hash,title:body.title}];return res.end(JSON.stringify({Hash:hash}))}res.statusCode=404;res.end("{}")});
  await new Promise(resolve=>server.listen(0,"127.0.0.1",resolve));
  try{const address=server.address();const client=new TorrServerClient(`http://127.0.0.1:${address.port}`);const first=await client.addTorrent({magnet:`magnet:?xt=urn:btih:${hash}&amp;dn=Movie`,title:"Movie"});assert.equal(first.hash,hash);assert.equal(first.alreadyExists,false);assert.equal(addBody.action,"add");assert.equal(addBody.save_to_db,true);assert.doesNotMatch(addBody.link,/&amp;/);const second=await client.addTorrent({magnet:`magnet:?xt=urn:btih:${hash}`,title:"Movie"});assert.equal(second.alreadyExists,true)}finally{await new Promise(resolve=>server.close(resolve))}
});

test("treats unavailable viewed history as empty on healthy MatriX servers",async()=>{
  const server=createServer((req,res)=>{if(req.url==="/viewed"){res.statusCode=500;return res.end("viewed database is empty")}res.statusCode=404;res.end()});
  await new Promise(resolve=>server.listen(0,"127.0.0.1",resolve));
  try{const address=server.address();const client=new TorrServerClient(`http://127.0.0.1:${address.port}`);assert.deepEqual(await client.listViewed(),[])}finally{await new Promise(resolve=>server.close(resolve))}
});

test("builds a MatriX stream URL with link and one-based file index",()=>{
  const client=new TorrServerClient("http://127.0.0.1:8090/");
  assert.equal(client.streamUrl("abc123",1,"Movie title.mkv"),"http://127.0.0.1:8090/stream/Movie%20title.mkv?link=abc123&index=1&play");
});

test("normalizes video files and detects season and episode",()=>{
  const files=normalizeTorrentFiles({file_stats:[{id:7,path:"Show/S02/Show.S02E03.mkv",length:123},{id:8,path:"Show/readme.txt",length:5}]});
  assert.deepEqual(files,[{id:7,path:"Show/S02/Show.S02E03.mkv",name:"Show.S02E03.mkv",length:123,season:2,episode:3}]);
});

test("detects Russian season folders and numbered episode files",()=>{
  const files=normalizeTorrentFiles({file_stats:[
    {id:733,path:"Смешарики/Смешарики Пин-код/Сезон 4/17. Собратья по разуму. Часть 1.mkv",length:521358362},
  ]});
  assert.deepEqual(files,[{id:733,path:"Смешарики/Смешарики Пин-код/Сезон 4/17. Собратья по разуму. Часть 1.mkv",name:"17. Собратья по разуму. Часть 1.mkv",length:521358362,season:4,episode:17}]);
});
