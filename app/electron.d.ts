export {};

declare global {
  interface Window { pirateCinema?:{
    choosePlayer:()=>Promise<string|null>;
    createBackup:()=>Promise<string|null>;restoreBackup:()=>Promise<boolean|null>;
    updateStatus:()=>Promise<UpdateState>;checkForUpdates:()=>Promise<UpdateState>;installUpdate:()=>Promise<void>;
    onUpdateState:(callback:(value:UpdateState)=>void)=>(()=>void);
  }; }
  type UpdateState={status:string;version:string|null;progress?:number;error:string|null};
}
