import {defineConfig} from "vite";
import react from "@vitejs/plugin-react";
import path from "node:path";
import {readFileSync} from "node:fs";

const androidVersion=readFileSync(new URL("./android/version.properties",import.meta.url),"utf8").match(/^versionName=(.+)$/m)?.[1]?.trim();
if(!androidVersion)throw new Error("Android versionName is missing");

export default defineConfig({
  root:"mobile",
  plugins:[react()],
  define:{__APP_VERSION__:JSON.stringify(androidVersion)},
  build:{outDir:"../dist-mobile",emptyOutDir:true},
  resolve:{alias:{"@":path.resolve(import.meta.dirname,".")}},
});
