import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname,resolve } from "node:path";
import { loadLocalEnv } from "./load-env.mjs";
const root=resolve(dirname(fileURLToPath(import.meta.url)),"..");loadLocalEnv(root);
const env={...process.env,WRANGLER_LOG_PATH:".wrangler/wrangler.log"};
const children=[];const torrServerUrl=env.TORRSERVER_URL??"http://127.0.0.1:8090";const torrServerExe=env.TORRSERVER_EXE?resolve(root,env.TORRSERVER_EXE):null;
async function torrServerOnline(){try{const response=await fetch(`${torrServerUrl.replace(/\/$/,"")}/echo`,{signal:AbortSignal.timeout(1200)});return response.ok}catch{return false}}
if(torrServerExe&&existsSync(torrServerExe)&&!await torrServerOnline()){
  const torrServer=spawn(torrServerExe,[],{cwd:dirname(torrServerExe),env,stdio:"ignore",windowsHide:true});children.push(torrServer);
  for(let attempt=0;attempt<20&&!await torrServerOnline();attempt++)await new Promise(resolve=>setTimeout(resolve,300));
}
children.push(spawn(process.execPath,[resolve(root,"server","api.mjs")],{cwd:root,env,stdio:"inherit"}),spawn(process.execPath,[resolve(root,"node_modules","vinext","dist","cli.js"),"dev"],{cwd:root,env,stdio:"inherit"}));
let stopping=false;function stop(code=0){if(stopping)return;stopping=true;for(const child of children)if(!child.killed)child.kill("SIGTERM");setTimeout(()=>process.exit(code),300).unref()}
for(const child of children)child.on("exit",(code)=>{if(!stopping&&code!==0)stop(code??1)});process.on("SIGINT",()=>stop(0));process.on("SIGTERM",()=>stop(0));
