import { version } from '../../package.json';

export interface WasmApi {
  default: (options?: unknown) => Promise<unknown>;
  encode_svg_with_size: (value: number, foreground: number, size: number) => string;
  decode_rgba: (width: number, height: number, pixels: Uint8Array) => string;
}
let instance: Promise<WasmApi> | undefined;
// Use the same cache key for the JS glue and WASM with the sized SVG export.
const wasmVersion=`${version}-svg-size`;
export function loadWasm(): Promise<WasmApi> {
  return instance ??= (async()=>{
    const url=`${import.meta.env.BASE_URL}wasm/int32_identicon_core.js?v=${wasmVersion}`;
    const api=await import(/* @vite-ignore */ url) as WasmApi;
    await api.default({module_or_path:`${import.meta.env.BASE_URL}wasm/int32_identicon_core_bg.wasm?v=${wasmVersion}`});return api;
  })();
}
