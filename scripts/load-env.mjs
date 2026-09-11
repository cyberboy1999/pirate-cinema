import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
export function loadLocalEnv(root=process.cwd()){
  const path=resolve(root,".env.local");if(!existsSync(path))return;
  for(const line of readFileSync(path,"utf8").split(/\r?\n/)){const trimmed=line.trim();if(!trimmed||trimmed.startsWith("#"))continue;const match=trimmed.match(/^([A-Z_][A-Z0-9_]*)=(.*)$/);if(!match||process.env[match[1]]!==undefined)continue;let value=match[2].trim();if((value.startsWith('"')&&value.endsWith('"'))||(value.startsWith("'")&&value.endsWith("'")))value=value.slice(1,-1);process.env[match[1]]=value}
}
