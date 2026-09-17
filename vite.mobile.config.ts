import {defineConfig} from "vite";
import react from "@vitejs/plugin-react";
import path from "node:path";

export default defineConfig({
  root:"mobile",
  plugins:[react()],
  build:{outDir:"../dist-mobile",emptyOutDir:true},
  resolve:{alias:{"@":path.resolve(import.meta.dirname,".")}},
});
