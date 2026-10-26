import {readFileSync} from 'node:fs';
import assert from 'node:assert/strict';
import {PNG} from 'pngjs';
import {initSync,encode_svg,decode_rgba} from '../web/public/wasm/int32_identicon_core.js';
initSync({module:readFileSync(new URL('../web/public/wasm/int32_identicon_core_bg.wasm',import.meta.url))});
const manifest=JSON.parse(readFileSync(new URL('../examples/manifest.json',import.meta.url)));
const started=performance.now();
for(const entry of manifest.examples){
  const png=PNG.sync.read(readFileSync(new URL(`../examples/${entry.png}`,import.meta.url)));
  const decoded=JSON.parse(decode_rgba(png.width,png.height,new Uint8Array(png.data)));
  assert.equal(decoded.value,entry.value);assert.equal(decoded.foreground,entry.foreground);
  assert.ok(encode_svg(entry.value,-1).startsWith('<svg'));
}
assert.throws(()=>decode_rgba(32,32,new Uint8Array(32*32*4).fill(255)));
// Use a real PNG as a small symbol inside a neutral desktop-like screen, so
// WASM also exercises color-frame location instead of only clean backgrounds.
const sample=manifest.examples[0];
const png=PNG.sync.read(readFileSync(new URL(`../examples/${sample.png}`,import.meta.url)));
const scene=new Uint8Array(640*480*4).fill(255);
for(let y=0;y<480;y++)for(let x=0;x<640;x++){
  const i=(y*640+x)*4;scene.set([207,210,196],i);
  if(x<25||(x<150&&y%23<3))scene.set([85,88,82],i);
  if(x>=220&&x<460&&y>=120&&y<360){
    const sx=Math.floor((x-220)*png.width/240),sy=Math.floor((y-120)*png.height/240);
    scene.set(png.data.subarray((sy*png.width+sx)*4,(sy*png.width+sx)*4+4),i);
  }
}
assert.equal(JSON.parse(decode_rgba(640,480,scene)).value,sample.value);
console.log(`WASM decoded all 10 real PNGs; blank image rejected (${(performance.now()-started).toFixed(1)} ms total in Node; not a phone benchmark)`);
