import {app,BrowserWindow,dialog,shell} from "electron";
import {spawn} from "node:child_process";
import {appendFileSync,copyFileSync,existsSync,mkdirSync,readFileSync,writeFileSync} from "node:fs";
import {dirname,join,resolve} from "node:path";
import {fileURLToPath} from "node:url";

const projectRoot=resolve(dirname(fileURLToPath(import.meta.url)),"..");
const children=[];
let quitting=false;
let mainWindow=null;

function log(message){try{const logDir=join(app.getPath("userData"),"logs");mkdirSync(logDir,{recursive:true});appendFileSync(join(logDir,"desktop.log"),`[${new Date().toISOString()}] ${message}\n`)}catch{/* Logging must never prevent the desktop app from starting. */}}
function childEnvironment(){const dataDir=join(app.getPath("userData"),"data");const bundledMpv=app.isPackaged?join(process.resourcesPath,"mpv","mpv.exe"):join(projectRoot,"vendor","mpv","mpv.exe");return {...process.env,ELECTRON_RUN_AS_NODE:"1",TORR_DATA_DIR:dataDir,TORRSERVER_URL:"http://127.0.0.1:8090",TORR_LOCAL_API_PORT:"3001",REMOTE_POSTERS:"1",WRANGLER_LOG_PATH:join(app.getPath("userData"),"wrangler.log"),MPV_PATH:process.env.MPV_PATH||bundledMpv,...(app.isPackaged?{FFMPEG_PATH:join(process.resourcesPath,"ffmpeg","ffmpeg.exe")}:{})}}
function runNode(script,args=[]){const child=spawn(process.execPath,[script,...args],{cwd:projectRoot,env:childEnvironment(),windowsHide:true,stdio:"ignore"});children.push(child);child.on("error",error=>log(`child error: ${error.message}`));child.on("exit",code=>log(`${script} exited with ${code}`));return child}
async function online(url,timeout=1200){try{return (await fetch(url,{signal:AbortSignal.timeout(timeout)})).ok}catch{return false}}
async function waitFor(url,timeoutMs=60000){const deadline=Date.now()+timeoutMs;while(Date.now()<deadline){if(await online(url,2000))return;await new Promise(resolve=>setTimeout(resolve,400))}throw new Error(`Сервис не ответил: ${url}`)}
function prepareTorrServer(){const targetDir=join(app.getPath("userData"),"torrserver");mkdirSync(targetDir,{recursive:true});const sourceDir=app.isPackaged?join(process.resourcesPath,"torrserver"):join(projectRoot,"bin");for(const name of ["config.db","settings.json","viewed.json","trackers.txt","rutor.ls"]){const source=join(sourceDir,name),target=join(targetDir,name);if(existsSync(source)&&!existsSync(target))copyFileSync(source,target)}const settingsPath=join(targetDir,"settings.json");try{const settings=JSON.parse(readFileSync(settingsPath,"utf8"));if(settings?.BitTorr){settings.BitTorr.EnableRutorSearch=true;writeFileSync(settingsPath,JSON.stringify(settings,null,2))}}catch(error){log(`TorrServer settings migration failed: ${error.message}`)}const executable=app.isPackaged?join(sourceDir,"TorrServer-windows-amd64.exe"):join(projectRoot,"bin","TorrServer-windows-amd64.exe");return {executable,targetDir}}
async function startServices(){if(!await online("http://127.0.0.1:8090/echo")){const {executable,targetDir}=prepareTorrServer();if(!existsSync(executable))throw new Error("TorrServer не найден в ресурсах приложения");const torrServer=spawn(executable,[],{cwd:targetDir,windowsHide:true,stdio:"ignore"});children.push(torrServer);torrServer.on("error",error=>log(`TorrServer error: ${error.message}`));await waitFor("http://127.0.0.1:8090/echo",30000)}if(!await online("http://127.0.0.1:3001/api/health"))runNode(join(projectRoot,"server","api.mjs"));if(!await online("http://127.0.0.1:3000/"))runNode(join(projectRoot,"node_modules","vinext","dist","cli.js"),["start"]);await Promise.all([waitFor("http://127.0.0.1:3001/api/health"),waitFor("http://127.0.0.1:3000/")])}
function createWindow(){const icon=join(projectRoot,"public","pirate-cinema-logo.png");mainWindow=new BrowserWindow({title:"Pirate Cinema",icon,width:1440,height:900,minWidth:1040,minHeight:700,show:false,backgroundColor:"#050505",autoHideMenuBar:true,webPreferences:{contextIsolation:true,nodeIntegration:false,sandbox:true,spellcheck:false}});mainWindow.webContents.setWindowOpenHandler(({url})=>{if(/^https?:/i.test(url))void shell.openExternal(url);return {action:"deny"}});mainWindow.webContents.on("will-navigate",(event,url)=>{if(!url.startsWith("http://127.0.0.1:3000/")&&!url.startsWith("http://localhost:3000/")){event.preventDefault();if(/^https?:/i.test(url))void shell.openExternal(url)}});mainWindow.once("ready-to-show",()=>mainWindow?.show());void mainWindow.loadURL("http://127.0.0.1:3000/")}
function stopChildren(){for(const child of children){try{if(!child.killed)child.kill("SIGTERM")}catch{/* A child may already have exited. */}}}

if(!app.requestSingleInstanceLock()){app.quit()}else{
  app.on("second-instance",()=>{if(mainWindow){if(mainWindow.isMinimized())mainWindow.restore();mainWindow.focus()}});
  app.on("before-quit",()=>{quitting=true;stopChildren()});
  app.on("window-all-closed",()=>{if(process.platform!=="darwin")app.quit()});
  app.on("activate",()=>{if(BrowserWindow.getAllWindows().length===0)createWindow()});
  app.whenReady().then(async()=>{app.setAppUserModelId("ru.piratecinema.desktop");try{await startServices();createWindow()}catch(error){log(error instanceof Error?error.stack??error.message:String(error));await dialog.showMessageBox({type:"error",title:"Pirate Cinema не запустился",message:"Не удалось запустить локальные сервисы",detail:error instanceof Error?error.message:String(error)});if(!quitting)app.quit()}});
}
