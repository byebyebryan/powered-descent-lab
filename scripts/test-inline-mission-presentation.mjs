import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';

const context=vm.createContext({
  URL,URLSearchParams,
  location:{href:'http://localhost/report.html',origin:'http://localhost',pathname:'/report.html'},
  document:{getElementById:()=>null}
});
for(const name of ['paired_model','study_model','comparison_review_model','mission_presentation'])
  vm.runInContext(readFileSync(new URL(`../pd-report/src/${name}.js`,import.meta.url),'utf8'),context,{filename:name});

const value=(key,label,n,unit)=>({key,label,value:n,unit,text:null});
const attempt=(approach,attempt_index,{outcome='landed',fuel=10,preview=`preview-${approach}.svg`,href=`flight-${attempt_index}.html`,cells='<td>legacy cell</td>',display=true}={})=>({
  approach,attempt_index,outcome,preview,href,caption:`attempt ${attempt_index}`,
  cells,display_values:display?[value('fuel_remaining','Fuel remaining',fuel,'kg')]:[]
});
const mission=attempts=>({id:'m',mission_index:11,group:'g',path:['g'],label:'Mission 11',physical_sha256:'a'.repeat(64),occurrence:0,attempts});
const render=(m,data,state,study=null,options={})=>context.PdMissionPresentation.render(m,state,data,study,options);

test('ambiguous legacy labels and multiple attempts remain a single selected run',()=>{
  const data={schema_version:1,labels:['First','Second'],missions:[]};
  const m=mission([attempt(0,1,{fuel:99}),attempt(1,7,{fuel:7}),attempt(1,8,{fuel:8})]);
  const state={view:'1',run:'8',mission:'m'};
  assert.equal(context.PdMissionPresentation.mode(data,null),'single');
  assert.equal(context.PdMissionPresentation.mode({...data,presentation:{schema_version:1,mode:'single'}},null),'single');
  const result=render(m,data,state);
  assert.equal(result.model.mode,'single');
  assert.equal(result.model.sides.length,1);
  assert.equal(result.model.sides[0].approach,1);
  assert.equal(result.model.sides[0].attempt.attempt_index,8);
  assert.equal(result.model.fields[0].values[0].value,8);
  assert.match(result.html,/mission-presentation-single/);
  assert.match(result.html,/Second/);
  assert.equal((result.html.match(/class="mission-run-card/g)||[]).length,1);
  assert.equal((result.html.match(/data-run-role=/g)||[]).length,1);
  assert.doesNotMatch(result.html,/Compare A|Compare B|Δ \(B−A\)/);
  assert.match(result.html,/data-mission-run="m"/);
});

test('explicit non-study comparison pins A and B and keeps a declared missing side visible',()=>{
  const data={schema_version:1,labels:['A label','B label'],presentation:{schema_version:1,mode:'comparison'}};
  const m=mission([attempt(0,1,{fuel:25})]);
  const result=render(m,data,{a:'0',b:'1',view:'0'});
  assert.equal(result.model.mode,'comparison');
  assert.deepEqual(Array.from(result.model.sides,s=>s.approach),[0,1]);
  assert.equal(result.model.sides[1].attempt,null);
  assert.equal(result.model.sides[1].outcome,'not-scheduled');
  assert.equal((result.html.match(/class="mission-run-card/g)||[]).length,2);
  assert.equal((result.html.match(/class="(?:run-preview )?mission-preview(?: |")/g)||[]).length,2);
  assert.equal((result.html.match(/data-run-role=/g)||[]).length,2);
  assert.match(result.html,/B · B label/);
  assert.match(result.html,/Not scheduled/);
  assert.match(result.html,/Unavailable/);
  assert.match(result.html,/Preview unavailable in this saved projection/);
  assert.equal(result.model.fields[0].values[0].value,25);
  assert.equal(result.model.fields[0].values[1],null);
  assert.doesNotMatch(result.html,/class="mission-delta/);
});

test('paired typed values stay in run rows and legacy HTML remains a separate disclosure',()=>{
  const data={schema_version:1,labels:['Run A','Run B'],presentation:{schema_version:1,mode:'comparison'}};
  const m=mission([attempt(0,1,{fuel:500}),attempt(1,2,{fuel:400})]);
  const result=render(m,data,{a:'0',b:'1',view:'0'});
  assert.deepEqual(Array.from(result.model.sides,s=>s.approach),[0,1]);
  assert.equal(result.model.fields[0].label,'Fuel remaining');
  assert.deepEqual(Array.from(result.model.fields[0].values,v=>v.value),[500,400]);
  assert.equal((result.html.match(/<tr data-run-role=/g)||[]).length,2);
  assert.match(result.html,/Run A/);
  assert.match(result.html,/Run B/);
  const legacy=mission([attempt(0,1,{display:false,cells:'<td>Old saved value</td>'})]);
  const old=render(legacy,{schema_version:1,labels:['One']},{view:'0'});
  assert.match(old.html,/Legacy saved run summary/);
  assert.match(old.html,/<td>Old saved value<\/td>/);
  assert.equal(old.model.fields.length,0);
  assert.match(old.html,/Structured measurements are unavailable/);
});

const studyFixture=(observations)=>({
  schema_version:1,id:'study',default_cohort:'primary',default_contrast:'base-candidate',default_metric:'fuel_used_kg',
  variants:[{id:'base',label:'Baseline'},{id:'candidate',label:'Candidate'}],
  contrasts:[{id:'base-candidate',baseline:'base',candidate:'candidate'}],
  cohorts:[{id:'primary',label:'Primary'}],
  metrics:[
    {id:'fuel_used_kg',label:'Fuel used',unit:'kg',lower_is_better:true,successful_pairs_only:true},
    {id:'coast_s',label:'Coast / preparation',unit:'s',lower_is_better:null,successful_pairs_only:false},
    {id:'duration_s',label:'Burn duration',unit:'s',lower_is_better:null,successful_pairs_only:false}
  ],observations
});
const observation=(variant,{outcome='landed',verified_landing=true,fuel=10,coast=3,duration=16}={})=>({
  mission:'m',cohort:'primary',variant,physical_sha256:'a'.repeat(64),occurrence:0,outcome,verified_landing,
  values:{fuel_used_kg:fuel,coast_s:coast,duration_s:duration},attributes:{}
});
const pairedMission=()=>mission([attempt(0,1,{fuel:100}),attempt(1,2,{fuel:90})]);
const studyState=(extra={})=>({cohort:'primary',contrast:'base-candidate',metric:'fuel_used_kg',basis:'strategy',view:'0',a:'0',b:'1',...extra});

test('study comparison shows the selected metric and declared timing fields on aligned run rows',()=>{
  const study=studyFixture([observation('base',{fuel:120,coast:3,duration:16}),observation('candidate',{fuel:100,coast:5,duration:12})]);
  const result=render(pairedMission(),{schema_version:1,labels:['Baseline','Candidate']},studyState(),study);
  const keys=Array.from(result.model.fields,f=>f.key);
  assert(keys.includes('study:fuel_used_kg'));
  assert(keys.includes('study:coast_s'));
  assert(keys.includes('study:duration_s'));
  assert.equal(result.model.studyPair.eligible,true);
  assert.equal(result.model.studyPair.delta,-20);
  assert.match(result.html,/Δ \(B−A\) · Fuel used: -20\.0000 kg/);
  assert.match(result.html,/Coast \/ preparation/);
  assert.match(result.html,/Burn duration/);
  assert.equal((result.html.match(/data-run-role=/g)||[]).length,2);
});

test('failed zero-fuel use is ineligible and cannot be presented as an efficiency win',()=>{
  const study=studyFixture([
    observation('base',{outcome:'crashed',verified_landing:false,fuel:0}),
    observation('candidate',{fuel:20})
  ]);
  const result=render(pairedMission(),{schema_version:1,labels:['Baseline','Candidate']},studyState(),study);
  assert.equal(result.model.studyPair.eligible,false);
  assert.equal(result.model.studyPair.delta,null);
  assert.match(result.html,/Δ \(B−A\) · Fuel used: Unavailable for this pair/);
  assert.match(result.html,/Crashed/);
  assert.match(result.html,/0 kg/);
});

test('sparse study pairing retains both declared run references and an unavailable metric cell',()=>{
  const study=studyFixture([observation('candidate',{fuel:30})]);
  const m=mission([attempt(1,2,{fuel:70})]);
  const result=render(m,{schema_version:1,labels:['Baseline','Candidate']},studyState(),study);
  assert.equal(result.model.sides.length,2);
  assert.equal(result.model.sides[0].attempt,null);
  assert.equal(result.model.sides[0].outcome,'not-scheduled');
  assert.equal(result.model.fields.find(f=>f.key==='study:fuel_used_kg').values[0],null);
  assert.match(result.html,/Not scheduled/);
  assert.match(result.html,/Unavailable/);
  assert.equal((result.html.match(/data-run-role=/g)||[]).length,2);
});

test('same-capture earlier edition keeps two labelled slots even when both use one attempt',()=>{
  context.PdComparisonReviewData={earlier:{same_captures_verified:true,scientific_changes:0}};
  const study=studyFixture([observation('candidate',{fuel:22})]);
  const result=render(pairedMission(),{schema_version:1,labels:['Baseline','Candidate']},studyState({basis:'earlier'}),study);
  assert.deepEqual(Array.from(result.model.sides,s=>s.approach),[1,1]);
  assert.equal(result.model.sides[0].attempt.attempt_index,result.model.sides[1].attempt.attempt_index);
  assert.match(result.html,/Earlier edition · Candidate/);
  assert.match(result.html,/Current edition · Candidate/);
  assert.match(result.html,/same captures and zero scientific changes; no behavioral improvement is claimed/);
  assert.equal((result.html.match(/class="mission-run-card/g)||[]).length,2);
  assert.equal((result.html.match(/class="mission-preview-image"/g)||[]).length,2);
  assert.equal((result.html.match(/data-run-role=/g)||[]).length,2);
  assert.equal(result.model.studyPair.delta,0);
});

test('optional evidence loads exact samples and replaced disclosures dispose or discard stale work',async()=>{
  const hosts=[];
  const document={createElement:()=>({className:'',textContent:''})};
  const vmContext=vm.createContext({AbortController,document});
  const samples={samples:[{simTimeS:0,xM:0,yM:0,attitudeRad:0,throttleFrac:0}]};
  let pending=null,mounts=0,destroys=0,lastRecords=null;
  vmContext.PdSavedAssets={
    payload:async()=>pending?pending.promise:samples,
    record:async()=>({record:{evidence:null}})
  };
  vmContext.PdFlightInspection={mountPair(host,records){mounts++;lastRecords=records;return {destroy(){destroys++;}};}};
  vm.runInContext(readFileSync(new URL('../pd-report/src/mission_inspection.js',import.meta.url),'utf8'),vmContext,{filename:'mission_inspection'});
  const side={key:'single',label:'Saved attempt',color:'#356a96',attempt:{attempt_index:3,href:'flight-3.html',outcome:'landed'}};
  const loaded=await vmContext.PdMissionInspection.loadAttempt(side.attempt,{label:side.label,color:side.color});
  assert.equal(loaded.evidence,null);
  assert.deepEqual(Array.from(loaded.goals),[]);
  assert.equal(loaded.data.samples.length,1);
  const host=()=>{const value={children:[],isConnected:true,replaceChildren(...children){this.children=children;},append(child){this.children.push(child);}};hosts.push(value);return value;};
  const first=host();
  await vmContext.PdMissionInspection.mountInline(first,[side]);
  assert.equal(mounts,1);
  assert.equal(lastRecords[0].evidence,null);
  const replacement=host();
  let resolvePending;
  pending={promise:new Promise(resolve=>{resolvePending=resolve;})};
  const loading=vmContext.PdMissionInspection.mountInline(replacement,[side]);
  assert.equal(destroys,1);
  assert.deepEqual(first.children,[]);
  await new Promise(resolve=>setImmediate(resolve));
  vmContext.PdMissionInspection.clear(replacement);
  resolvePending(samples);
  await loading;
  assert.equal(mounts,1,'a disclosure closed during a delayed load must never mount stale samples');
  assert.equal(destroys,1);
  assert.deepEqual(replacement.children,[]);
});
