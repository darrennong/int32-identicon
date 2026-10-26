import{test}from'node:test';
import assert from'node:assert/strict';
import{guideCrop}from'./capture.ts';
const close=(a:number,b:number)=>assert.ok(Math.abs(a-b)<1e-7,`${a} != ${b}`);
test('square preview crops the visible guide from a landscape camera',()=>{
  const c=guideCrop(1280,720,360,360)!;
  close(c.sx,352);close(c.sy,72);close(c.sw,576);close(c.sh,576);
  assert.equal(c.width,640);assert.equal(c.height,640);
});
test('short preview preserves the rectangular guide instead of cropping an unseen square',()=>{
  const c=guideCrop(1280,720,360,240)!;
  close(c.sx,208);close(c.sy,72);close(c.sw,864);close(c.sh,576);
  assert.equal(c.width,640);assert.equal(c.height,427);
});
test('portrait camera and invalid layout dimensions',()=>{
  const c=guideCrop(720,1280,360,480)!;
  close(c.sx,72);close(c.sy,256);close(c.sw,576);close(c.sh,768);
  assert.equal(c.width,480);assert.equal(c.height,640);
  assert.equal(guideCrop(0,720,360,360),null);
  assert.equal(guideCrop(1280,720,0,360),null);
  assert.equal(guideCrop(1280,720,Infinity,360),null);
});
