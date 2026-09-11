import {createServer} from "node:http";
const port=Number(process.env.MOCK_TORR_PORT??18090);
const releases=[
  ["Сёгун / Shogun [S01] (2024) WEB-DL-HEVC 2160p | 4K | HDR","72.40 GB",842,"a".repeat(40)],
  ["Сёгун / Shogun [S01] (2024) WEB-DL 1080p | LostFilm","31.87 GB",689,"b".repeat(40)],
  ["Сёгун / Shogun [S01] (2024) WEB-DL 1080p | Red Head Sound","29.39 GB",612,"c".repeat(40)],
  ["Сёгун / Shogun [S01] (2024) WEB-DL 720p | D, P, A","25.83 GB",588,"d".repeat(40)],
  ["Сёгун / Shogun [S01] (2024) WEB-DLRip | LostFilm","8.17 GB",502,"e".repeat(40)],
].map(([Title,Size,Seed,Hash])=>({Title,Size,Seed,Peer:2,Hash,Magnet:`magnet:?xt=urn:btih:${Hash}`,Year:2024,VideoQuality:Title.includes("2160p")?305:Title.includes("1080p")?203:Title.includes("720p")?103:0}));
const server=createServer(async(req,res)=>{const url=new URL(req.url??"/",`http://127.0.0.1:${port}`);res.setHeader("content-type","application/json");if(url.pathname==="/echo")return res.end("MatriX.135");if(url.pathname==="/search")return res.end(JSON.stringify(releases));if(url.pathname==="/torrents"||url.pathname==="/viewed")return res.end("[]");res.statusCode=404;res.end("{}");});
server.listen(port,"127.0.0.1",()=>console.log(`[mock-torrserver] http://127.0.0.1:${port}`));
for(const signal of ["SIGINT","SIGTERM"])process.on(signal,()=>server.close(()=>process.exit(0)));
