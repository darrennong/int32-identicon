import './style.css';
import { loadWasm, type WasmApi } from './wasm-api';
import { Consensus, type Candidate } from './consensus';

const el=<T extends HTMLElement>(id:string)=>document.getElementById(id) as T;
const integer=el<HTMLInputElement>('integer');
const foreground=el<HTMLSelectElement>('foreground');
const symbol=el<HTMLDivElement>('symbol');
const genStatus=el<HTMLParagraphElement>('generate-status');
const scanStatus=el<HTMLParagraphElement>('scan-status');
const output=el<HTMLOutputElement>('scan-result');
const video=el<HTMLVideoElement>('camera');
const start=el<HTMLButtonElement>('camera-start');
const stop=el<HTMLButtonElement>('camera-stop');
const buttons=['download-svg','download-png','self-check'].map(id=>el<HTMLButtonElement>(id));
const worker=new Worker(new URL('./worker.ts',import.meta.url),{type:'module'});
type Response={result:Candidate|null;elapsed:number;error?:string};
const jobs=new Map<number,{resolve:(r:Response)=>void;reject:(e:Error)=>void}>();
let nextJob=0;
worker.onmessage=(event: MessageEvent<Response&{id:number}>)=>{
  const job=jobs.get(event.data.id);jobs.delete(event.data.id);job?.resolve(event.data);
};
worker.onerror=()=>{for(const job of jobs.values())job.reject(Error('识别 Worker 加载失败，请刷新页面。'));jobs.clear();};
function decodeImage(data:ImageData):Promise<Response> {
  return new Promise((resolve,reject)=>{
    const id=++nextJob;
    jobs.set(id,{resolve,reject});
    const buffer=data.data.buffer as ArrayBuffer;
    worker.postMessage({id,width:data.width,height:data.height,buffer},[buffer]);
  });
}

let api:WasmApi;
let currentSvg='';
let currentValue=123456789;
let stream:MediaStream|null=null;
let timer:ReturnType<typeof setTimeout>|undefined;
let epoch=0;
let frame=0;
const consensus=new Consensus();
function parseInteger():number|null {
  if(!/^-?\d+$/.test(integer.value))return null;
  const v=Number(integer.value);
  return Number.isInteger(v)&&v>=-2147483648&&v<=2147483647?v:null;
}
function update() {
  if(!api)return;
  const v=parseInteger();
  integer.setAttribute('aria-invalid',String(v===null));
  buttons.forEach(b=>b.disabled=v===null);
  if(v===null){genStatus.textContent='请输入范围内的整数。';return;}
  currentValue=v;
  currentSvg=api.encode_svg(v,Number(foreground.value));
  symbol.innerHTML=currentSvg;
  el('value-caption').textContent=String(v);
  genStatus.textContent='图标已生成，可下载或在另一块屏幕上扫描。';
}
integer.addEventListener('input',update);
foreground.addEventListener('change',update);
el('random').addEventListener('click',()=>{integer.value=String(crypto.getRandomValues(new Int32Array(1))[0]);update();});

function download(blob:Blob,name:string) {
  const url=URL.createObjectURL(blob),a=document.createElement('a');
  a.href=url;a.download=name;a.click();setTimeout(()=>URL.revokeObjectURL(url),1500);
}
async function loadImage(blob:Blob):Promise<HTMLImageElement> {
  const url=URL.createObjectURL(blob),image=new Image();
  try {await new Promise<void>((resolve,reject)=>{image.onload=()=>resolve();image.onerror=()=>reject(Error('图片无法读取。'));image.src=url;});return image;}
  finally{URL.revokeObjectURL(url);}
}
function imagePixels(image:HTMLImageElement,maxSize=1024):ImageData {
  const scale=Math.min(1,maxSize/Math.max(image.naturalWidth,image.naturalHeight));
  const canvas=document.createElement('canvas');
  canvas.width=Math.max(32,Math.round(image.naturalWidth*scale));canvas.height=Math.max(32,Math.round(image.naturalHeight*scale));
  const context=canvas.getContext('2d',{willReadFrequently:true})!;
  context.fillStyle='white';context.fillRect(0,0,canvas.width,canvas.height);
  context.drawImage(image,0,0,canvas.width,canvas.height);
  return context.getImageData(0,0,canvas.width,canvas.height);
}
el('download-svg').addEventListener('click',()=>download(new Blob([currentSvg],{type:'image/svg+xml'}),`int32-${currentValue}.svg`));
el('download-png').addEventListener('click',async()=>{
  const value=currentValue,svg=currentSvg;
  try {
    const image=await loadImage(new Blob([svg],{type:'image/svg+xml'}));
    const canvas=document.createElement('canvas');canvas.width=640;canvas.height=640;
    const context=canvas.getContext('2d')!;context.imageSmoothingEnabled=false;context.drawImage(image,0,0,640,640);
    canvas.toBlob(blob=>{if(blob)download(blob,`int32-${value}.png`);},'image/png');
  }catch(error){genStatus.textContent=String(error);}
});
el('self-check').addEventListener('click',async()=>{
  const value=currentValue;
  genStatus.textContent='正在从渲染后的像素解码…';
  try {
    const image=await loadImage(new Blob([currentSvg],{type:'image/svg+xml'}));
    const response=await decodeImage(imagePixels(image));
    if(parseInteger()!==value)return;
    genStatus.textContent=response.result?.value===value
      ?`像素解码通过：${response.result.value} · ${response.elapsed.toFixed(1)} ms`
      :'未能从当前图标解码。';
  }catch(error){genStatus.textContent=String(error);}
});

function stopCamera() {
  epoch++;
  if(timer)clearTimeout(timer);
  stream?.getTracks().forEach(track=>track.stop());stream=null;
  video.srcObject=null;
  el('camera-placeholder').hidden=false;
  start.disabled=false;stop.disabled=true;consensus.reset();output.value='—';
  el('timing').textContent='';
}
function showResponse(response:Response,camera:boolean,captured:number=0,id:number=0) {
  el('timing').textContent=`本帧识别 ${response.elapsed.toFixed(1)} ms`;
  if(camera) {
    const c=consensus.push(response.result,captured,id);
    output.value=c.confirmed?String(c.confirmed.value):'—';
    scanStatus.textContent=c.confirmed?`连续 ${c.count} 帧一致 · V1 · 前景色 ${c.confirmed.foreground}`
      :response.result?`已识别 ${c.count}/3 帧，继续保持稳定。`:'未确认图标，请调整距离、倾角与白色留边。';
  } else {
    output.value=response.result?String(response.result.value):'—';
    scanStatus.textContent=response.result?`静态图片解码成功（单帧）· V1 · 修复 ${response.result.repairedBytes} 字节`:'未找到可校验的图标。请使用白色背景、完整色卡和清晰图片。';
  }
}
start.addEventListener('click',async()=>{
  stopCamera();const session=epoch;start.disabled=true;
  scanStatus.textContent='正在申请摄像头权限…';
  try {
    if(!window.isSecureContext||!navigator.mediaDevices?.getUserMedia)throw Error('摄像头需要 HTTPS 和支持的浏览器。');
    const next=await navigator.mediaDevices.getUserMedia({video:{facingMode:{ideal:'environment'},width:{ideal:1280},height:{ideal:720}},audio:false});
    if(epoch!==session){next.getTracks().forEach(track=>track.stop());return;}
    stream=next;video.srcObject=stream;await video.play();
    if(epoch!==session)return;
    stop.disabled=false;el('camera-placeholder').hidden=true;
    scanStatus.textContent='将图标及白色留边放入取景框。';
    const canvas=document.createElement('canvas');canvas.width=640;canvas.height=640;
    const context=canvas.getContext('2d',{willReadFrequently:true})!;
    let lastVideoTime=-1;
    let lastCaptureTime=0;
    const capture=async()=>{
      if(session!==epoch||!stream)return;
      if(video.readyState>=2&&video.currentTime!==lastVideoTime) {
        lastVideoTime=video.currentTime;
        const side=Math.min(video.videoWidth,video.videoHeight)*0.8;
        const captured=performance.now(),id=++frame;
        lastCaptureTime=captured;
        context.drawImage(video,(video.videoWidth-side)/2,(video.videoHeight-side)/2,side,side,0,0,640,640);
        try {
          const response=await decodeImage(context.getImageData(0,0,640,640));
          if(session===epoch)showResponse(performance.now()-captured<=700?response:{...response,result:null},true,captured,id);
        }catch(error){if(session===epoch){stopCamera();scanStatus.textContent=String(error);}return;}
      } else if(performance.now()-lastCaptureTime>700) {
        consensus.reset();output.value='—';scanStatus.textContent='等待新的摄像头画面…';
      }
      if(session===epoch)timer=setTimeout(capture,250);
    };
    void capture();
  } catch(error) {
    if(session===epoch){stopCamera();scanStatus.textContent=error instanceof DOMException&&error.name==='NotAllowedError'?'摄像头权限未开启。可在浏览器设置中允许，或选择图片。':String(error);}
  }
});
stop.addEventListener('click',()=>{stopCamera();scanStatus.textContent='摄像头已停止。';});
el<HTMLInputElement>('image-file').addEventListener('change',async(event)=>{
  stopCamera();const session=epoch;
  const file=(event.target as HTMLInputElement).files?.[0];if(!file)return;
  if(file.size>20*1024*1024){scanStatus.textContent='请选择小于 20 MB 的图片。';return;}
  scanStatus.textContent='正在本地读取图片…';
  try {
    const image=await loadImage(file);
    if(image.naturalWidth>4096||image.naturalHeight>4096)throw Error('图片长边请不超过 4096 像素。');
    const response=await decodeImage(imagePixels(image));
    if(session===epoch)showResponse(response,false);
  }catch(error){if(session===epoch)scanStatus.textContent=String(error);}
});
function selectTab(name:'generate'|'scan') {
  if(name==='generate')stopCamera();
  for(const id of ['generate','scan']) {
    el(id).hidden=id!==name;
    el(`${id}-tab`).setAttribute('aria-selected',String(id===name));
    el(`${id}-tab`).tabIndex=id===name?0:-1;
  }
}
for(const name of ['generate','scan'] as const) {
  el(`${name}-tab`).addEventListener('click',()=>selectTab(name));
  el(`${name}-tab`).addEventListener('keydown',event=>{
    if(event.key==='ArrowLeft'||event.key==='ArrowRight') {
      event.preventDefault();const next=name==='generate'?'scan':'generate';selectTab(next);el(`${next}-tab`).focus();
    }
  });
}
document.addEventListener('visibilitychange',()=>{if(document.hidden){stopCamera();scanStatus.textContent='页面离开前台，摄像头已停止。';}});
window.addEventListener('pagehide',()=>stopCamera());
selectTab('generate');
loadWasm().then(loaded=>{api=loaded;update();}).catch(()=>{genStatus.textContent='WASM 加载失败，请检查网络后刷新。';});
