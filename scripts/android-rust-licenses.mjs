// Build artifact only: preserve the license/copyright files shipped with locked Rust crates.
import {execFileSync} from "node:child_process";
import {copyFileSync,existsSync,mkdirSync,readdirSync,writeFileSync} from "node:fs";
import path from "node:path";
const root=path.resolve(import.meta.dirname,".."),output=path.join(root,"android/app/src/main/assets/licenses/rust");
const metadata=JSON.parse(execFileSync("cargo",["metadata","--manifest-path",path.join(root,"android/rust/Cargo.toml"),"--locked","--format-version","1","--filter-platform","aarch64-linux-android"],{encoding:"utf8",maxBuffer:20*1024*1024}));
mkdirSync(output,{recursive:true});
const notices=[];
for(const crate of metadata.packages){
  const directory=path.dirname(crate.manifest_path),destination=path.join(output,`${crate.name}-${crate.version}`);
  const files=readdirSync(directory,{withFileTypes:true}).filter(entry=>entry.isFile()&&/^(?:licen[cs]e|copying|notice|unlicense)(?:$|[._-])/i.test(entry.name)).map(entry=>entry.name);
  if(crate.license_file&&existsSync(path.join(directory,crate.license_file))&&!crate.license_file.includes("..")&&!files.includes(crate.license_file))files.push(crate.license_file);
  if(files.length){mkdirSync(destination,{recursive:true});for(const file of files)copyFileSync(path.join(directory,file),path.join(destination,path.basename(file)))}
  notices.push({name:crate.name,version:crate.version,license:crate.license,repository:crate.repository,licenseFiles:files.map(file=>`${crate.name}-${crate.version}/${path.basename(file)}`)});
}
writeFileSync(path.join(output,"INDEX.json"),JSON.stringify(notices,null,2));
console.log(`Prepared license notices for ${notices.length} locked Rust crates`);
