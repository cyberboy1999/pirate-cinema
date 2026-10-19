// Physical debug WebView check. Add the owned-series fixture first; open its file chooser.
// adb forward tcp:9223 localabstract:webview_devtools_remote_<app-pid>
import assert from "node:assert/strict";
const targets=await(await fetch("http://127.0.0.1:9223/json")).json();
const target=targets.find(item=>item.type==="page"&&item.url.startsWith("https://localhost"));
assert.ok(target,"App debug WebView must be open");
const socket=new WebSocket(target.webSocketDebuggerUrl),pending=new Map();let sequence=0;
await new Promise((resolve,reject)=>{socket.addEventListener("open",resolve,{once:true});socket.addEventListener("error",reject,{once:true})});
socket.addEventListener("message",({data})=>{const message=JSON.parse(data);pending.get(message.id)?.(message)});
async function evaluate(expression){const id=++sequence;const response=await new Promise((resolve,reject)=>{const timer=setTimeout(()=>{pending.delete(id);reject(new Error("CDP timeout"))},15000);pending.set(id,message=>{clearTimeout(timer);pending.delete(id);resolve(message)});socket.send(JSON.stringify({id,method:"Runtime.evaluate",params:{expression,awaitPromise:true,returnByValue:true,userGesture:true}}))});assert.ok(!response.error&&!response.result?.exceptionDetails,JSON.stringify(response));return response.result.result.value}
async function until(expression,timeout=30000){const end=Date.now()+timeout;while(Date.now()<end){const value=await evaluate(expression);if(value)return value;await new Promise(resolve=>setTimeout(resolve,300))}throw new Error(`Condition timed out: ${expression}`)}
const snapshot=`(()=>{const v=document.querySelector('video');return v&&{time:v.currentTime,width:v.videoWidth,height:v.videoHeight,ready:v.readyState,paused:v.paused}})()`;
try {
  await until(`[...document.querySelectorAll('button')].some(b=>['Воспроизвести','Сначала'].includes(b.textContent.trim()))`);
  await evaluate(`[...document.querySelectorAll('button')].find(b=>['Воспроизвести','Сначала'].includes(b.textContent.trim())).click()`);
  await until(`document.querySelector('video')?.readyState===4`,45000);
  console.log("PASS initial MKV playback",await evaluate(snapshot));
  await evaluate(`(async()=>{const v=document.querySelector('video');v.currentTime=3;window.fixtureAudio=new AudioContext();const source=fixtureAudio.createMediaElementSource(v);window.fixtureAnalyser=fixtureAudio.createAnalyser();fixtureAnalyser.fftSize=8192;source.connect(fixtureAnalyser);fixtureAnalyser.connect(fixtureAudio.destination);await fixtureAudio.resume();await v.play()})()`);
  const frequency=`(()=>{const bins=new Float32Array(fixtureAnalyser.frequencyBinCount);fixtureAnalyser.getFloatFrequencyData(bins);let peak=0;for(let i=1;i<bins.length;i++)if(bins[i]>bins[peak])peak=i;return bins[peak]>-80?peak*fixtureAudio.sampleRate/fixtureAnalyser.fftSize:0})()`;
  await until(`Math.abs(${frequency}-440)<12`);console.log("PASS decoded first audio ~440 Hz");
  await evaluate(`[...document.querySelectorAll('.art-selector-item')].find(b=>b.textContent.includes('English-880Hz')).click()`);
  await until(`Math.abs(${frequency}-880)<12`);console.log("PASS switched decoded audio ~880 Hz");
  await evaluate(`[...document.querySelectorAll('.art-selector-item')].find(b=>b.textContent==='360P').click()`);
  await until(`document.querySelector('video').videoHeight===360`);
  await evaluate(`[...document.querySelectorAll('.art-selector-item')].find(b=>b.textContent==='240P').click()`);
  await until(`document.querySelector('video').videoHeight===240&&document.querySelector('video').readyState===4`);
  console.log("PASS 360p -> 240p",await evaluate(snapshot));
  await evaluate(`document.querySelector('video').currentTime=42`);
  await until(`document.querySelector('video').currentTime>43&&document.querySelector('video').readyState===4`);
  console.log("PASS seek resumes frames",await evaluate(snapshot));
  await evaluate(`(()=>{const s=[...document.querySelectorAll('select')].at(-1);s.value='1';s.dispatchEvent(new Event('change',{bubbles:true}))})()`);
  await until(`document.querySelector('video')?.readyState===4&&[...document.querySelectorAll('select')].at(-1).value==='1'`,45000);
  console.log("PASS episode switch",await evaluate(snapshot));
  assert.ok(await evaluate(`Object.values(JSON.parse(localStorage.getItem('pirate-cinema-mobile-history'))).some(h=>h.fileId===0&&h.positionMs>=42000)`));
  console.log("PASS per-file history retained");
  await evaluate(`(()=>{const s=[...document.querySelectorAll('select')].at(-1);s.value='0';s.dispatchEvent(new Event('change',{bubbles:true}))})()`);
  await until(`document.querySelector('video')?.readyState===4&&document.querySelector('video').currentTime>=42`,45000);
  console.log("PASS first episode resumes saved position",await evaluate(snapshot));
} finally {await evaluate(`window.fixtureAudio?.close()`).catch(()=>{});socket.close()}
