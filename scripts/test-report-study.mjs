import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
const c=vm.createContext({});
for(const name of ['study_model','paired_model'])vm.runInContext(readFileSync(new URL(`../pd-report/src/${name}.js`,import.meta.url),'utf8'),c);
const model=c.PdStudyReviewModel;
const study={default_cohort:'primary',default_contrast:'ab',default_metric:'fuel',cohorts:[{id:'primary'},{id:'boundary'}],contrasts:[{id:'ab',baseline:'a',candidate:'b'},{id:'ba',baseline:'b',candidate:'a'}],metrics:[{id:'fuel',successful_pairs_only:true}],condition_axes:['family','sink'],observations:[],claims:[{detail:'Frozen result on 4 originals'}]};
const missions=[];
for(let i=0;i<4;i++){
  const m={id:`m${i}`,physical_sha256:`physical${i}`,occurrence:0,mission_index:i,direction:0,attempts:[]};missions.push(m);
  for(const [variant,success] of [['a',i<2],['b',i===0||i===2]]){
    const outcome=success?'landed':'crashed';m.attempts.push({approach:variant==='a'?0:1,outcome});
    study.observations.push({mission:m.id,cohort:'primary',variant,physical_sha256:m.physical_sha256,occurrence:0,outcome,verified_landing:success,values:{fuel:variant==='a'?100:success?99.9999:0},attributes:{family:'vertical',sink:'low'}});
  }
}
const state=model.normalize({},study);
test('paired outcome transitions partition the declared originals without crash fuel credit',()=>{
  const result=model.summary(missions,state,study);
  assert.equal(JSON.stringify(result.counts),JSON.stringify({preserved:1,gained:1,lost:1,missed:1,unpaired:0}));
  assert.equal(result.pairs.length,1);assert(Math.abs(result.pairs[0].delta+.0001)<1e-9);
  assert.equal(model.matches(missions[1],{...state,change:'lost'},study),true);
  assert.equal(model.matches(missions[1],{...state,change:'lost',lo:-1000},study),false);
});
test('cohort roles, missing variants, zero differences and precise ranges remain distinct',()=>{
  const sparse=JSON.parse(JSON.stringify(study));sparse.observations=sparse.observations.filter(o=>!(o.mission==='m3'&&o.variant==='b'));
  assert.equal(model.pair(missions[3],state,sparse).category,'unpaired');
  assert.equal(model.summary(missions,{...state,cohort:'boundary'},study).total,0);
  assert.equal(model.matches(missions[0],{...state,lo:0,hi:0},study),false);
  assert.equal(model.matches(missions[0],{...state,hi:0},study),true);
  assert.equal(model.matches(missions[0],{...state,cell:'braking|low'},study),false);
});
test('exploration does not mutate frozen declarations, and reversing a contrast reverses change',()=>{
  const before=JSON.stringify(study);assert.equal(model.pair(missions[1],{...state,contrast:'ba'},study).category,'gained');
  const selected=missions.filter(m=>model.matches(m,{...state,change:'lost'},study));assert.equal(selected.length,1);
  assert.equal(JSON.stringify(study),before);
  const repeat={...missions[0],id:'repeat',occurrence:1};const repeated=JSON.parse(before);repeated.observations.push(...repeated.observations.filter(o=>o.mission==='m0').map(o=>({...o,mission:'repeat',occurrence:1})));
  assert.equal(model.summary([...missions,repeat],state,repeated).total,5);
});
test('the maintained mission predicate intersects study selections and stays unchanged without a study',()=>{
  const legacy={q:'',group:'',direction:'all',approach:'all',outcome:'all'};
  assert(c.PdPairedReviewModel.matches(missions[0],legacy));
  c.PdStudyProjection=study;
  assert(c.PdPairedReviewModel.matches(missions[1],{...legacy,...state,change:'lost'}));
  assert(!c.PdPairedReviewModel.matches(missions[0],{...legacy,...state,change:'lost'}));
  delete c.PdStudyProjection;
});
