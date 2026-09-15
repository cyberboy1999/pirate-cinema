import type { Metadata } from "next";
import { Geist } from "next/font/google";
import { headers } from "next/headers";
import {version as appVersion} from "../package.json";
import "./globals.css";
import "./local.css";
const geist=Geist({variable:"--font-geist",subsets:["latin","cyrillic"]});
export async function generateMetadata():Promise<Metadata>{
  const requestHeaders=await headers();
  const host=requestHeaders.get("x-forwarded-host")??requestHeaders.get("host")??"localhost:3000";
  const protocol=requestHeaders.get("x-forwarded-proto")??(host.startsWith("localhost")?"http":"https");
  const image=`${protocol}://${host}/og.png`;
  const title=`Pirate Cinema ${appVersion} — локальная медиатека`;
  const description="Поиск, медиатека и локальный стриминг через TorrServer.";
  return {title,description,icons:{icon:"/favicon.png",shortcut:"/favicon.png",apple:"/pirate-cinema-logo.png"},openGraph:{title,description,images:[{url:image,width:1536,height:864,alt:"Pirate Cinema personal media library"}]},twitter:{card:"summary_large_image",title,description,images:[image]}};
}
export default function RootLayout({children}:Readonly<{children:React.ReactNode}>){return <html lang="ru"><body className={geist.variable}>{children}</body></html>}
