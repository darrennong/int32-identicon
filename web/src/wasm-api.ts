export interface WasmApi {
  default: (options?: unknown) => Promise<unknown>;
  encode_svg: (value: number, foreground: number) => string;
  decode_rgba: (width: number, height: number, pixels: Uint8Array) => string;
}
let instance: Promise<WasmApi> | undefined;
export function loadWasm(): Promise<WasmApi> {
  return instance ??= (async()=>{
    const url=`${import.meta.env.BASE_URL}wasm/int32_identicon_core.js?v=${version}`;
    const api=await import(/* @vite-ignore */ url) as WasmApi;
    await api.default({module_or_path:`${import.meta.env.BASE_URL}wasm/int32_identicon_core_bg.wasm?v=${version}`});return api;
  })();
}
import { version } from '../../package.json';
