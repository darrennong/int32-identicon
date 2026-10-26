// Independent conformance encoder. RS parity is solved from its two syndromes,
// rather than copying the Rust polynomial-division encoder.
import { writeFileSync, readFileSync } from 'node:fs';
const mask = [0xa5,0x3c,0x96,0xf0,0x69,0x5a,0xc3,0x87,0x1e];
const foregrounds = [0,1,4,8,9,12,13,14,15];
const mul = (a,b) => {
  let r = 0;
  while (b) { if (b&1) r ^= a; a = (a<<1) ^ (a&128 ? 0x11d : 0); b >>>= 1; }
  return r;
};
const pow = (a,n) => { let r=1; while(n--) r=mul(r,a); return r; };
const crc16 = bytes => {
  let crc=0xffff;
  for (const byte of bytes) {
    crc ^= byte<<8;
    for(let i=0;i<8;i++) crc=((crc<<1) ^ (crc&0x8000 ? 0x1021 : 0)) & 0xffff;
  }
  return crc;
};
const hex = a => Buffer.from(a).toString('hex').toUpperCase();
export function encode(value, fg=foregrounds[(value>>>0)%9]) {
  const data=Buffer.alloc(7);
  data[0]=0x10|fg;
  data.writeInt32BE(value,1);
  data.writeUInt16BE(crc16(data.subarray(0,5)),5);
  const s0=[...data].reduce((s,b)=>s^b,0);
  const s1=[...data,0,0].reduce((s,b)=>mul(s,2)^b,0);
  const p=mul(s0^s1,pow(3,254)), q=s0^p;
  const codeword=[...data,p,q];
  return {value,foreground:fg,crcHex:hex(data.subarray(5,7)),codewordHex:hex(codeword),maskedHex:hex(codeword.map((b,i)=>b^mask[i]))};
}
if (crc16(Buffer.from('123456789')) !== 0x29b1) throw Error('CRC check failed');
let seed=0x49333244;
const random=Array.from({length:10},()=>{seed^=seed<<13;seed^=seed>>>17;seed^=seed<<5;return seed|0;});
const vectors={protocol:1,crcCheck:'29B1',vectors:[-2147483648,2147483647,-1,0,1,...random].map(v=>encode(v))};
const target=new URL('../tests/fixtures/v1-vectors.json',import.meta.url);
if(process.argv.includes('--write')) {
  writeFileSync(target,JSON.stringify(vectors,null,2)+'\n');
  console.log('wrote 15 independent V1 conformance vectors');
} else {
  if(JSON.stringify(JSON.parse(readFileSync(target)))!==JSON.stringify(vectors)) throw Error('conformance vectors changed');
  const manifest=JSON.parse(readFileSync(new URL('../examples/manifest.json',import.meta.url)));
  for(const sample of manifest.examples) {
    const expected=encode(sample.value,sample.foreground);
    for(const key of ['crcHex','codewordHex','maskedHex']) if(sample[key]!==expected[key]) throw Error(`sample mismatch: ${sample.value} ${key}`);
    if(sample.decodedFromPng!==sample.value) throw Error('pixel decode mismatch');
  }
  console.log('independent encoder agrees with 15 vectors and all 10 example codewords');
}
