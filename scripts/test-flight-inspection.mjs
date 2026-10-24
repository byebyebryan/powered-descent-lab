import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import {readFileSync} from 'node:fs';
const c=vm.createContext({});vm.runInContext(readFileSync(new URL('../pd-report/src/flight_model.js',import.meta.url),'utf8'),c);
const m=c.PdFlightModel;
test('inspection uses recorded sample clocks and marks ended flights without synthesizing continuation',()=>{
  const samples=[{simTimeS:2,throttleFrac:0},{simTimeS:2.5,throttleFrac:1},{simTimeS:3,throttleFrac:.4}];
  assert.equal(m.at(samples,2.499).index,0);assert.equal(m.at(samples,2.5).index,1);assert(m.at(samples,1).before);assert(m.at(samples,3.001).ended);assert(!m.at(samples,3).ended);assert.equal(m.at([],2).sample,null);
});
test('display unwraps attitude across the branch cut without changing native values',()=>{
  const samples=[179,-179,-175].map(v=>({simTimeS:v,attitudeRad:v*Math.PI/180,targetAttitudeRad:-179*Math.PI/180}));const before=JSON.stringify(samples),out=m.angles(samples);
  assert(Math.abs(out[1].actual-181)<1e-9);assert(Math.abs(out[0].target-181)<1e-9);assert.equal(JSON.stringify(samples),before);
  const fixed=m.angles([179,-179].map(v=>({attitudeRad:v*Math.PI/180,targetAttitudeRad:0})));assert(Math.abs(fixed[0].target-fixed[1].target)<1e-9);
});
test('saved payload access tolerates quoted braces and escaped text without executing report scripts',()=>{
  const data={samples:[{text:'a } \\" b',simTimeS:1}],value:3};assert.equal(JSON.stringify(m.extract('const reportData = '+JSON.stringify(data)+';evil()')),JSON.stringify(data));assert.throws(()=>m.extract('no payload'));
  assert.equal(JSON.stringify(m.extract('const marker="const reportData = ";\n    const reportData = '+JSON.stringify(data)+';\n')),JSON.stringify(data));
});
test('paired camera covers all trajectories and pad edges on identical physical axes',()=>{
  const records=[{data:{samples:[{xM:-1000,yM:400},{xM:0,yM:0}],missionDetails:{targetPad:{centerXM:0,widthM:36,surfaceYM:0}}}},{data:{samples:[{xM:3000,yM:800}],missionDetails:{}}}];const b=m.bounds(records);assert(b.xmin< -1000&&b.xmax>3000&&b.ymin<0&&b.ymax>800);
});
