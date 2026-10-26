import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { resolve, join } from 'node:path';
import { homedir } from 'node:os';

const version='0.2.129'; // MUST match crates/core/Cargo.toml.
const windows=process.platform==='win32';
const target=windows?'x86_64-pc-windows-msvc':process.platform==='darwin'
  ?`${process.arch==='arm64'?'aarch64':'x86_64'}-apple-darwin`:'x86_64-unknown-linux-musl';
const stem=`wasm-bindgen-${version}-${target}`;
const executable=resolve('.tools',stem,`wasm-bindgen${windows?'.exe':''}`);
function run(cmd,args) {
  const p=spawnSync(cmd,args,{stdio:'inherit',shell:false});
  if(p.error) throw p.error;
  if(p.status!==0) throw Error(`${cmd} exited with ${p.status}`);
}
if(!existsSync(executable)) {
  mkdirSync('.tools',{recursive:true});
  const url=`https://github.com/wasm-bindgen/wasm-bindgen/releases/download/${version}/${stem}.tar.gz`;
  const get=async u=>{const r=await fetch(u);if(!r.ok)throw Error(`download failed: ${r.status} ${u}`);return r;};
  const archive=Buffer.from(await (await get(url)).arrayBuffer());
  const expected=(await (await get(`${url}.sha256sum`)).text()).trim().split(/\s+/)[0].toLowerCase();
  if(createHash('sha256').update(archive).digest('hex')!==expected) throw Error('wasm-bindgen SHA-256 mismatch');
  const file=resolve('.tools',`${stem}.tar.gz`);writeFileSync(file,archive);
  run('tar',['-xzf',file,'-C',resolve('.tools')]);
}
const localCargo=join(homedir(),'.cargo','bin',`cargo${windows?'.exe':''}`);
run(existsSync(localCargo)?localCargo:'cargo',['build','--locked','--release','--target','wasm32-unknown-unknown','-p','int32-identicon-core']);
mkdirSync('web/public/wasm',{recursive:true});
run(executable,['target/wasm32-unknown-unknown/release/int32_identicon_core.wasm','--target','web','--out-dir','web/public/wasm','--out-name','int32_identicon_core','--no-typescript']);
console.log(`built Rust/WASM with wasm-bindgen ${version}`);
