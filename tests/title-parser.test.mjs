import test from "node:test";
import assert from "node:assert/strict";
import { localizedTorrentTitle, matchConfidence, parseTorrentTitle } from "../server/title-parser.mjs";

test("extracts a clean title and year from a release name",()=>{
  const parsed=parseTorrentTitle("Dune.Part.Two.2024.2160p.WEB-DL.DDP5.1.Atmos.DV.HDR.H.265.mkv");
  assert.equal(parsed.title,"Dune Part Two");
  assert.equal(parsed.year,2024);
});

test("removes episode and release group noise without destroying the title",()=>{
  const parsed=parseTorrentTitle("Shogun.S01E08.1080p.WEBRip.LostFilm.mkv");
  assert.equal(parsed.title,"Shogun");
});

test("scores an exact title and year as an automatic match",()=>{
  const parsed=parseTorrentTitle("Dune.Part.Two.2024.REMUX.mkv");
  assert.ok(matchConfidence(parsed,{title:"Dune: Part Two",year:2024,type:"movie"})>=.85);
  assert.ok(matchConfidence(parsed,{title:"Dune",year:1984,type:"movie"})<.85);
});

test("uses the Russian release title for display",()=>{
  assert.equal(localizedTorrentTitle("Сёгун / Shogun [S01] (2024) WEB-DL"),"Сёгун");
  assert.equal(localizedTorrentTitle("Tusk (2014) BDRip"),null);
});
