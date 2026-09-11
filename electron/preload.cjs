/* eslint-disable @typescript-eslint/no-require-imports */
const {contextBridge,ipcRenderer}=require("electron");

contextBridge.exposeInMainWorld("pirateCinema",{
  choosePlayer:()=>ipcRenderer.invoke("player:choose"),
  createBackup:()=>ipcRenderer.invoke("backup:create"),restoreBackup:()=>ipcRenderer.invoke("backup:restore"),
  updateStatus:()=>ipcRenderer.invoke("update:status"),checkForUpdates:()=>ipcRenderer.invoke("update:check"),installUpdate:()=>ipcRenderer.invoke("update:install"),
  onUpdateState:callback=>{const listener=(_event,value)=>callback(value);ipcRenderer.on("update:state",listener);return()=>ipcRenderer.removeListener("update:state",listener)}
});
