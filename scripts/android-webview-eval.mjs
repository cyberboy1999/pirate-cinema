// Debug APK integration checks on an ADB-forwarded WebView, never a release API.
const [expression,timeoutText]=process.argv.slice(2);
if(!expression)throw new Error("Pass a JavaScript expression for the connected debug WebView");
const timeoutMs=Math.min(300_000,Math.max(1_000,Number(timeoutText)||30_000));
const targets=await(await fetch("http://127.0.0.1:9223/json")).json();
const target=targets.find(item=>item.type==="page"&&item.url.startsWith("https://localhost"));
if(!target)throw new Error("Pirate Cinema WebView not found");
const socket=new WebSocket(target.webSocketDebuggerUrl);
await new Promise((resolve,reject)=>{socket.addEventListener("open",resolve,{once:true});socket.addEventListener("error",reject,{once:true})});
const timeout=setTimeout(()=>{socket.close();process.exitCode=1;console.error(`WebView check timed out after ${timeoutMs} ms`)},timeoutMs);
socket.addEventListener("message",({data})=>{const message=JSON.parse(data);if(message.id===1){console.log(JSON.stringify(message.result??message.error,null,2));clearTimeout(timeout);socket.close();if(message.result?.exceptionDetails)process.exitCode=1}});
socket.send(JSON.stringify({id:1,method:"Runtime.evaluate",params:{expression,awaitPromise:true,returnByValue:true}}));
