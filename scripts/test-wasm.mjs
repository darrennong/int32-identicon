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
console.log(`WASM decoded all 10 real PNGs; blank image rejected (${(performance.now()-started).toFixed(1)} ms total in Node; not a phone benchmark)`);
