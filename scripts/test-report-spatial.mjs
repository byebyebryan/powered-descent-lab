import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
const context=vm.createContext({});
vm.runInContext(readFileSync(new URL('../pd-report/src/spatial_model.js',import.meta.url),'utf8'),context);
const {scalarSegments,vectorIndices,extent}=context.PdSpatialModel;
test('dense colour geometry retains every tiny segment with bounded trace count',()=>{
  const x=Array.from({length:10001},(_,i)=>i/1000),y=x.map(v=>v*v),v=x.map(v=>v/10);
  const before=JSON.stringify([x,y,v]);
  const groups=scalarSegments(x,y,v,{min:0,max:1});
  assert(groups.length<=32);
  assert.equal(groups.reduce((n,g)=>n+g.x.length/3,0),10000);
  for(const g of groups)for(let i=0;i<g.x.length;i+=3){assert.equal(g.x[i+2],null);assert(g.x[i+1]>g.x[i]);}
  assert.equal(JSON.stringify([x,y,v]),before);
});
test('invalid samples break paths and constant/empty scalar data remain explicit',()=>{
  assert.equal(scalarSegments([0,1,2],[0,NaN,2],[0,1,2],{min:0,max:2}).length,0);
  assert.equal(scalarSegments([],[],[],{min:0,max:1}).length,0);
  assert.equal(extent([0,0]).min,0);assert.equal(extent([0,0]).max,1);
  assert.equal(scalarSegments([0,1],[0,1],[0,0],{min:0,max:1})[0].bin,0);
});
test('vector density follows screen distance, zoom, viewport and powered samples',()=>{
  const samples=Array.from({length:1000},(_,i)=>({xM:i,yM:0,attitudeRad:0,throttleFrac:i<10?0:1}));
  const a=vectorIndices(samples,(x,y)=>[x/10,y]);
  const b=vectorIndices(samples,(x,y)=>[x,y]);
  assert(a.length<b.length);assert(b.length<=18);assert(a.every(i=>i>=10));
  assert.equal(vectorIndices(samples,()=>null).length,0);
  assert.equal(vectorIndices(samples,()=>[1,1]).length,1);
  for(let i=1;i<b.length;i++)assert(samples[b[i]].xM-samples[b[i-1]].xM>=42);
});
