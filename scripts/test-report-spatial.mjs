import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
const context=vm.createContext({});
vm.runInContext(readFileSync(new URL('../pd-report/src/spatial_model.js',import.meta.url),'utf8'),context);
const {scalarSegments,vectorIndices,extent}=context.PdSpatialModel;
test('vector tail is the recorded point and head extends along actual attitude at every scale',()=>{
  for(const scale of [.2,1,7])for(const angle of [0,Math.PI/2,-Math.PI/2,Math.PI,.72,-.72]){
    const x={l2p:v=>v*scale+80,p2l:p=>(p-80)/scale},y={l2p:v=>400-v*scale,p2l:p=>(400-p)/scale};
    const s={xM:12,yM:23,attitudeRad:angle,throttleFrac:.7},a=context.PdSpatialModel.vectorAnnotation(s,{xaxis:x,yaxis:y});
    assert(Math.abs(x.l2p(a.x)+a.ax-x.l2p(s.xM))<1e-9);assert(Math.abs(y.l2p(a.y)+a.ay-y.l2p(s.yM))<1e-9);
    assert(Math.abs((x.l2p(a.x)-x.l2p(s.xM))-Math.sin(angle)*16.8)<1e-9);assert(Math.abs((y.l2p(a.y)-y.l2p(s.yM))+Math.cos(angle)*16.8)<1e-9);
  }
});
test('vector sampling includes late powered behavior instead of exhausting the budget early',()=>{
  const s=Array.from({length:1000},(_,i)=>({xM:i*10,yM:0,attitudeRad:i>900?.7:0,throttleFrac:1}));
  const selected=vectorIndices(s,(x,y)=>[x,y]);assert(selected.includes(0));assert(selected.includes(999));assert(selected.some(i=>i>900));
});
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
