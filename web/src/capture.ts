/** Map the CSS guide through centered object-fit:cover into source pixels.
 * Preserve its aspect ratio, including short viewports and device rotation. */
export function guideCrop(sourceWidth:number,sourceHeight:number,viewWidth:number,viewHeight:number){
  if([sourceWidth,sourceHeight,viewWidth,viewHeight].some(v=>!Number.isFinite(v)||v<=0))return null;
  const scale=Math.max(viewWidth/sourceWidth,viewHeight/sourceHeight);
  const sw=viewWidth*0.8/scale,sh=viewHeight*0.8/scale;
  const outputScale=640/Math.max(sw,sh);
  return {sx:(sourceWidth-sw)/2,sy:(sourceHeight-sh)/2,sw,sh,
    width:Math.max(32,Math.round(sw*outputScale)),height:Math.max(32,Math.round(sh*outputScale))};
}
