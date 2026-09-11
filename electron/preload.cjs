/* eslint-disable @typescript-eslint/no-require-imports */
const {contextBridge,ipcRenderer}=require("electron");

contextBridge.exposeInMainWorld("pirateCinema",{choosePlayer:()=>ipcRenderer.invoke("player:choose")});
