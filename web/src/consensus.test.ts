import { test } from 'node:test';
import assert from 'node:assert/strict';
import { Consensus, type Candidate } from './consensus.ts';
const candidate: Candidate={value:-2147483648,foreground:0,repairedBytes:0,version:1};
test('requires three distinct consecutive frames, including Int32 minimum',()=>{
  const c=new Consensus();
  assert.equal(c.push(candidate,100,1).confirmed,null);
  assert.equal(c.push(candidate,200,2).confirmed,null);
  assert.equal(c.push(candidate,200,2).confirmed,null);
  assert.equal(c.push(candidate,300,3).confirmed?.value,-2147483648);
});
test('uncertainty, time gaps, and different results reset confirmation',()=>{
  const c=new Consensus();
  c.push(candidate,100,1);c.push(candidate,200,2);
  assert.equal(c.push(null,300,3).count,0);
  assert.equal(c.push(candidate,400,4).confirmed,null);
  assert.equal(c.push(candidate,1200,5).count,1);
  assert.equal(c.push({...candidate,value:2147483647},1300,6).count,1);
  c.reset();assert.equal(c.push(candidate,1,1).count,1);
});
