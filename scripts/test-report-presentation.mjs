import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';

const context=vm.createContext({});
vm.runInContext(readFileSync(new URL('../pd-report/src/paired_model.js',import.meta.url),'utf8'),context);
const {matches,normalizeScope,pageMissions,pageSize,visibleBranches}=context.PdPairedReviewModel;
const mission={id:'mission-0',mission_index:5,direction:-1,path:['family-0','stratum-0'],label:'Value noise · left · stratum 0',group:'coverage-0',physical_sha256:'abcdef01234',attempts:[{approach:0,outcome:'timed-out'},{approach:1,outcome:'landed'},{approach:2,outcome:'unsupported'}]};
const all={q:'',group:'',outcome:'all',approach:'all',direction:'all'};

test('coverage focus and selected outcome approach are exact intersections',()=>{
  assert(matches(mission,all));
  assert(matches(mission,{...all,group:'coverage-0',outcome:'landed',approach:'1'}));
  assert(!matches(mission,{...all,group:'coverage-1',outcome:'landed',approach:'1'}));
  assert(!matches(mission,{...all,outcome:'landed',approach:'0'}));
  assert(matches(mission,{...all,outcome:'unsupported'}));
  assert(matches(mission,{...all,outcome:'misses',approach:'0'}));
  assert(!matches(mission,{...all,outcome:'misses',approach:'1'}));
});

test('numeric search identifies a mission; identity and terrain search stay literal',()=>{
  assert(matches(mission,{...all,q:'5'}));
  assert(!matches(mission,{...all,q:'0'}));
  assert(matches(mission,{...all,q:'ABCDEF'}));
  assert(matches(mission,{...all,q:'value noise · left'}));
  assert(!matches(mission,{...all,q:'<script>'}));
});

test('selection retains repeat occurrences and leaves the paired attempts intact',()=>{
  const repeat={...mission,id:'mission-1',occurrence:1};
  const originals=JSON.stringify([mission,repeat]);
  const selected=[mission,repeat].filter(m=>matches(m,{...all,outcome:'timed-out',approach:'0'}));
  assert.equal(selected.length,2);
  assert.equal(selected[0].attempts.length,3);
  assert.equal(JSON.stringify([mission,repeat]),originals);
});

test('direction is an optional intersection within a merged terrain stratum',()=>{
  const right={...mission,id:'mission-2',direction:1,group:'coverage-1'};
  assert(matches(mission,{...all,group:'stratum-0'}));
  assert(matches(right,{...all,group:'stratum-0'}));
  assert(matches(mission,{...all,group:'stratum-0',direction:'-1',outcome:'landed',approach:'1'}));
  assert(!matches(right,{...all,group:'stratum-0',direction:'-1'}));
  assert(!matches(mission,{...all,direction:'-1',outcome:'landed',approach:'0'}));
  assert(!matches(mission,{...all,direction:'0'}));
});

test('legacy coverage selectors preserve their exact side and do not mutate inventory or input',()=>{
  const groups=[{id:'coverage-0',branch:'stratum-0',direction:-1}];
  const original={...all,group:'coverage-0'};const before=JSON.stringify(original);
  const state=normalizeScope(original,groups);
  assert.equal(state.group,'stratum-0');assert.equal(state.direction,'-1');
  assert(matches(mission,state));
  assert(!matches({...mission,direction:1,group:'coverage-1'},state));
  assert.equal(JSON.stringify(original),before);
  assert.equal(normalizeScope({...state,direction:'all'},groups).direction,'all');
  assert.equal(normalizeScope(original,[{id:'coverage-0'}]).group,'coverage-0');
});

test('bounded pages cover every scheduled occurrence in order, including repeats',()=>{
  const inventory=Array.from({length:128},(_,i)=>({...mission,id:`mission-${i}`,occurrence:i}));
  const before=JSON.stringify(inventory);const visited=[];
  for(let page=1;page<=Math.ceil(inventory.length/pageSize);page++){
    const slice=pageMissions(inventory,page);
    assert(slice.items.length<=pageSize);assert.equal(slice.total,128);
    visited.push(...slice.items.map(m=>m.id));
  }
  assert.deepEqual(visited,inventory.map(m=>m.id));
  assert.equal(JSON.stringify(inventory),before);
});

test('a detail return reveals its exact occurrence on the correct page',()=>{
  const inventory=Array.from({length:128},(_,i)=>({...mission,id:`mission-${i}`,occurrence:i}));
  const slice=pageMissions(inventory,1,'mission-24');
  assert.equal(slice.page,3);assert.equal(slice.start,25);assert.equal(slice.end,36);
  assert.equal(slice.items[0].id,'mission-24');
  assert.equal(pageMissions(inventory,2,'missing').page,2);
});

test('page bounds remain usable for invalid URLs, last pages and empty filters',()=>{
  const inventory=Array.from({length:13},(_,i)=>({...mission,id:`mission-${i}`}));
  for(const page of [-1,0,'NaN',Infinity])assert.equal(pageMissions(inventory,page).page,1);
  assert.equal(pageMissions(inventory,99).page,2);
  assert.equal(pageMissions(inventory,2).items.length,1);
  const empty=pageMissions([],99);
  assert.equal(empty.page,1);assert.equal(empty.pages,0);assert.equal(empty.start,0);assert.equal(empty.end,0);
  assert.equal(empty.items.length,0);
});

test('detail navigation rejects an external return URL and filters neighbours',async()=>{
  const links=Object.fromEntries(['previous','next'].map(k=>[k,{hidden:true}]));
  const back={};const nav={dataset:{},querySelector:s=>links[s.includes('previous')?'previous':'next']};
  const m={...mission,attempts:mission.attempts.map(a=>({...a,href:`details/mission-0-a${a.approach}.html`}))};
  const neighbour={...m,id:'mission-2',mission_index:13,attempts:m.attempts.map(a=>({...a,href:`details/mission-2-a${a.approach}.html`}))};
  const excluded={...neighbour,id:'mission-1',group:'coverage-1',direction:1};
  const run=async review=>{
    const body={dataset:{reviewRoot:'/edition/',reviewMission:'mission-0',reviewApproach:'0'}};
    const ctx=vm.createContext({URL,URLSearchParams,location:{origin:'http://localhost:8000',search:'?review='+encodeURIComponent(review)},document:{body,querySelector:s=>s==='[data-back-review]'?back:nav},fetch:async()=>({ok:true,json:async()=>({schema_version:1,missions:[m,excluded,neighbour]})})});
    vm.runInContext(readFileSync(new URL('../pd-report/src/paired_model.js',import.meta.url),'utf8'),ctx);
    vm.runInContext(readFileSync(new URL('../pd-report/src/detail_navigation.js',import.meta.url),'utf8'),ctx);
    await new Promise(r=>setImmediate(r));
  };
  await run('/edition/report.html?group=coverage-0&approach=0&outcome=timed-out');
  assert.equal(nav.dataset.contextReady,'true');
  assert(links.previous.hidden);
  assert.equal(new URL(links.next.href,'http://localhost').pathname,'/edition/details/mission-2-a0.html');
  assert(new URL(links.next.href,'http://localhost').searchParams.get('review').includes('group=coverage-0'));
  await run('/edition/report.html?group=stratum-0&direction=-1&approach=0&outcome=timed-out');
  assert.equal(new URL(links.next.href,'http://localhost').pathname,'/edition/details/mission-2-a0.html');
  assert.equal(new URL(links.next.href,'http://localhost').searchParams.get('review').includes('direction=-1'),true);
  await run('https://outside.example/report.html');
  assert.equal(back.href,'/edition/report.html#mission-0');
});

test('family and merged stratum focus preserve descendants and repeats',()=>{
  const nested={...mission,path:['family-0','stratum-0']};
  assert(matches(nested,{...all,group:'family-0'}));
  assert(matches(nested,{...all,group:'stratum-0',outcome:'landed',approach:'1'}));
  assert(!matches(nested,{...all,group:'stratum-1'}));
  assert(matches({...nested,occurrence:1},{...all,group:'family-0'}));
});

test('browsing every V2 group keeps only its hierarchy without accumulating siblings',()=>{
  const branches=[{id:'family-0',parent:null},...Array.from({length:50},(_,i)=>[
    {id:`morphology-${i}`,parent:'family-0'},
    {id:`band-${i}`,parent:`morphology-${i}`}
  ]).flat()];
  const before=JSON.stringify(branches);
  for(let i=0;i<50;i++){
    const visible=visibleBranches(branches,`band-${i}`);
    assert.deepEqual(Array.from(visible,b=>b.id),['family-0',`morphology-${i}`,`band-${i}`]);
  }
  assert.equal(JSON.stringify(branches),before);
  assert.equal(visibleBranches(branches,'band-49',true),branches);
});

test('overview, group picker and legacy strata retain their complete hierarchy',()=>{
  const branches=[{id:'family-0',parent:null},{id:'stratum-0',parent:'family-0'},
    {id:'family-1',parent:null},{id:'stratum-1',parent:'family-1'}];
  assert.equal(visibleBranches(branches),branches);
  assert.equal(visibleBranches(branches,'missing'),branches);
  assert.equal(visibleBranches(branches,'stratum-1',true),branches);
  assert.deepEqual(Array.from(visibleBranches(branches,'stratum-1'),b=>b.id),['family-1','stratum-1']);
  assert(matches({...mission,path:['family-1','stratum-1']},{...all,group:'stratum-1'}));
});

test('report navigation groups known input collections, independently of controller results',()=>{
  const ctx=vm.createContext({});
  vm.runInContext(readFileSync(new URL('../pd-report/src/mission_navigation.js',import.meta.url),'utf8'),ctx);
  const {family}=ctx.PdMissionNavigationModel;
  assert.equal(family('/reports/eval/terminal_bot_lab_suite/index.html'),'terminal');
  assert.equal(family('/reports/eval/transfer_route_angle_radius_suite/index.html'),'transfer');
  assert.equal(family('/reports/eval/transfer_waypoint_sequence_route_angle_radius_smoke/index.html'),'waypoint');
  assert.equal(family('/procedural-learning/run/held-out-evaluation/report.html'),'procedural');
  assert.equal(family('/reports/eval/procedural-learning/presentation/report.html'),'procedural');
  assert.equal(family('/eval/planner_v2_random_terrain/capture/index.html'),'planner');
  assert.equal(family('/reports/eval/unknown_study/index.html'),'other');
});
