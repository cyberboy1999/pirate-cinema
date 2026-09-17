import type {CapacitorConfig} from "@capacitor/cli";

const config:CapacitorConfig={
  appId:"com.piratecinema.app",
  appName:"Pirate Cinema",
  webDir:"dist-mobile",
  android:{allowMixedContent:true},
};

export default config;
