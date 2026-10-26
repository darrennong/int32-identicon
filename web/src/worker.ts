import { loadWasm } from './wasm-api';
const wasm=loadWasm();
self.onmessage=async(event: MessageEvent<{id:number;width:number;height:number;buffer:ArrayBuffer}>)=>{
  const {id,width,height,buffer}=event.data;
  const started=performance.now();
  try {
    const api=await wasm;
    const result=JSON.parse(api.decode_rgba(width,height,new Uint8Array(buffer)));
    self.postMessage({id,result,elapsed:performance.now()-started});
  } catch(error) {
    self.postMessage({id,result:null,error:String(error),elapsed:performance.now()-started});
  }
};
