import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import {readFileSync} from 'node:fs';
const c=vm.createContext({document:{getElementById:()=>null}});
for(const file of ['comparison_review_model','study_model'])vm.runInContext(readFileSync(new URL('../pd-report/src/'+file+'.js',import.meta.url),'utf8'),c);
const m=c.PdComparisonReviewModel;
test('metric change and native target attainment are independent and unrounded',()=>{
  assert.equal(m.change({eligible:true,delta:-.00001},{lower_is_better:true}),'better');assert.equal(m.target({outcome:'crashed',verified_landing:false}),'unmet');
  assert.equal(m.change({eligible:true,delta:0},{lower_is_better:true}),'tie');assert.equal(m.change({eligible:false,delta:0},{lower_is_better:true}),'unavailable');assert.equal(m.target({outcome:'unsupported',verified_landing:false}),'unavailable');assert.equal(m.change({eligible:true,delta:-1},{lower_is_better:null}),'descriptive');
});
test('earlier edition comparison is available only for an explicitly proved same-capture join',()=>{
  const study={default_cohort:'c',default_contrast:'ab',default_metric:'fuel',cohorts:[{id:'c'}],contrasts:[{id:'ab',baseline:'a',candidate:'b'}],metrics:[{id:'fuel',successful_pairs_only:true}],observations:[]};const mission={id:'m'};
  for(const [variant,value] of [['a',10],['b',9]])study.observations.push({mission:'m',cohort:'c',variant,outcome:'landed',verified_landing:true,values:{fuel:value}});
  const state=c.PdStudyReviewModel.normalize({basis:'earlier'},study);assert.equal(state.basis,'strategy');assert.equal(c.PdStudyReviewModel.pair(mission,state,study).delta,-1);
  c.PdComparisonReviewData={earlier:{same_captures_verified:true}};const prior=c.PdStudyReviewModel.normalize({basis:'earlier'},study);assert.equal(prior.basis,'earlier');assert.equal(c.PdStudyReviewModel.pair(mission,prior,study).delta,0);delete c.PdComparisonReviewData;
});
test('regression and unmet-target filters intersect the exact original population',()=>{
  const study={contrasts:[{id:'ab',baseline:'a',candidate:'b'}],metrics:[{id:'fuel',lower_is_better:true,successful_pairs_only:true}],condition_axes:[],observations:[{mission:'m',cohort:'c',variant:'a',outcome:'landed',verified_landing:true,values:{fuel:10}},{mission:'m',cohort:'c',variant:'b',outcome:'landed',verified_landing:true,values:{fuel:11}}]};
  assert(c.PdStudyReviewModel.matches({id:'m'},{cohort:'c',contrast:'ab',metric:'fuel',metric_change:'worse',target_status:'met'},study));assert(!c.PdStudyReviewModel.matches({id:'m'},{cohort:'c',contrast:'ab',metric:'fuel',metric_change:'worse',target_status:'unmet'},study));
});
test('an unavailable pinned comparison does not silently select another mission pair',()=>{
  c.PdComparisonReviewData={};const s={default_cohort:'c',default_contrast:'ok',default_metric:'fuel',cohorts:[{id:'c'}],contrasts:[{id:'ok'}],metrics:[{id:'fuel'}]};const state=c.PdStudyReviewModel.normalize({contrast:'missing'},s);assert(state.comparison_error.includes('missing'));assert(!c.PdStudyReviewModel.matches({id:'m'},state,s));delete c.PdComparisonReviewData;
});
